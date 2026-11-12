//! HTML & CSS's code: HTML files shown as web pages, as layouts (API 1.15).
//!
//! Both layouts are for `*.html`, `*.htm`, and `*.xhtml` files, chosen with
//! their buttons on the file's tab or **View: Reopen Editor With…**:
//!
//! - **HTML Preview** (`preview`): the file's editor with the page beside
//!   it, in one tab.
//! - **HTML Page** (`page`): the page by itself, in a tab of its own.
//!
//! The page is a file page: Flareite serves the open folder's files to it as
//! they are in the editor, unsaved changes included, and shows it again as
//! they're edited (in any tab) or change on disk, keeping where it was
//! scrolled. Its scripts run, sandboxed as extensions' pages are; it reaches
//! the network only if this extension is allowed to. Pages of files are
//! shown only in a trusted folder.
//!
//! It also types closing tags in HTML documents (API 1.19, see [`tags`]):
//! `</div>` after `<div>`, and the rest of `</div>` after `</`.

mod tags;

use flareite_extension_api::layouts::{
  self, Direction, EditorPane, FillReason, Layout, LayoutNode, LayoutTab, NodeKind, Pane,
  PaneContent, PaneHeader,
};
use flareite_extension_api::types::{
  Activation, CommandContext, DocumentChange, DocumentId, DocumentInfo, LogLevel, Position, Range,
};
use flareite_extension_api::{Guest, export_extension, host, language_features};

/// The command that fills both layouts' tabs in.
const FILL: &str = "flareite.html-css.preview";
/// The layout of the page by itself.
const PAGE: &str = "page";
/// The command that answers what to insert as `>` or `/` is typed.
const AUTO_INSERT: &str = "flareite.html-css.autoInsert";

fn file_name(path: &str) -> &str {
  path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn pane(
  id: &str,
  parent: Option<&str>,
  content: PaneContent,
  header: Option<PaneHeader>,
) -> LayoutNode {
  LayoutNode {
    id: id.into(),
    parent: parent.map(Into::into),
    size: 1.0,
    kind: NodeKind::Pane(Pane {
      content,
      header,
      scroll_group: None,
    }),
  }
}

/// The tab of an HTML file and its preview.
fn preview(path: &str) -> Layout {
  let name = file_name(path);
  Layout {
    title: format!("{name} (Preview)"),
    nodes: vec![
      LayoutNode {
        id: "split".into(),
        parent: None,
        size: 1.0,
        kind: NodeKind::Split(Direction::Row),
      },
      pane(
        "source",
        Some("split"),
        PaneContent::Editor(EditorPane {
          uri: path.into(),
          minimap: true,
          folding: true,
          quick_diff: true,
        }),
        None,
      ),
      pane(
        "page",
        Some("split"),
        PaneContent::FilePage(path.into()),
        Some(PaneHeader {
          title: "Preview".into(),
          description: Some(name.into()),
          buttons: Vec::new(),
        }),
      ),
    ],
    documents: Vec::new(),
    buttons: Vec::new(),
    breakpoints: Vec::new(),
    main: Some("source".into()),
  }
}

/// The tab of an HTML file's page by itself.
fn page(path: &str) -> Layout {
  Layout {
    title: format!("{} (Page)", file_name(path)),
    nodes: vec![pane("page", None, PaneContent::FilePage(path.into()), None)],
    documents: Vec::new(),
    buttons: Vec::new(),
    breakpoints: Vec::new(),
    main: Some("page".into()),
  }
}

/// Fills a tab in: its key is the file's path.
fn fill() {
  let Some(request) = layouts::filling() else {
    return;
  };
  if !matches!(request.reason, FillReason::Opened | FillReason::Restored) {
    return;
  }
  let tab = LayoutTab {
    layout: request.tab.layout,
    key: request.tab.key,
  };
  let layout = if tab.layout == PAGE {
    page(&tab.key)
  } else {
    preview(&tab.key)
  };
  if let Err(problem) = layouts::set_layout(&tab, &layout) {
    host::log(
      LogLevel::Error,
      &format!("can't show the page of {}: {problem:?}", tab.key),
    );
  }
}

/// Answers an auto-insert request with a closing tag, if one is due.
fn auto_insert() {
  let Some(request) = language_features::inserting() else {
    return;
  };
  let Some(trigger) = request.trigger.chars().next() else {
    return;
  };
  let range = Range {
    start: Position { line: 0, column: 0 },
    end: request.position,
  };
  // The text up to the cursor, as it was when the trigger was typed.
  let Ok(before) = host::read_range(request.document, range) else {
    return;
  };
  if before.version != request.version {
    return;
  }
  // SVG is XML; HTML is the other language this inserts for.
  let svg = host::document(request.document)
    .is_ok_and(|document| document.language.as_deref() == Some("svg"));
  let dialect = if svg {
    tags::Dialect::Xml
  } else {
    tags::Dialect::Html
  };
  if let Some((text, cursor)) = tags::auto_insert(&before.text, trigger, dialect)
    && let Err(problem) = language_features::set_insert(&text, cursor as u32)
  {
    host::log(
      LogLevel::Error,
      &format!("can't insert a closing tag: {problem:?}"),
    );
  }
}

struct HtmlCss;

impl Guest for HtmlCss {
  fn activate(_: Activation) -> Result<(), String> {
    Ok(())
  }

  fn deactivate() {}

  fn run_command(command: String, _: CommandContext) -> Result<(), String> {
    match command.as_str() {
      FILL => fill(),
      AUTO_INSERT => auto_insert(),
      other => return Err(format!("unknown command {other}")),
    }
    Ok(())
  }

  fn document_opened(_: DocumentInfo) {}

  fn document_changed(_: DocumentChange) {}

  fn document_closed(_: DocumentId) {}
}

export_extension!(HtmlCss);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_preview_is_beside_the_file() {
    let layout = preview("C:\\site\\index.html");
    assert_eq!(layout.title, "index.html (Preview)");
    assert_eq!(layout.nodes.len(), 3);
    assert!(matches!(
      &layout.nodes[2].kind,
      NodeKind::Pane(Pane {
        content: PaneContent::FilePage(path),
        ..
      }) if path == "C:\\site\\index.html"
    ));
  }

  #[test]
  fn the_page_has_a_tab_of_its_own() {
    let layout = page("/site/index.html");
    assert_eq!(layout.title, "index.html (Page)");
    assert_eq!(layout.nodes.len(), 1);
    assert_eq!(layout.nodes[0].parent, None);
    assert!(matches!(
      &layout.nodes[0].kind,
      NodeKind::Pane(Pane {
        content: PaneContent::FilePage(path),
        ..
      }) if path == "/site/index.html"
    ));
  }
}
