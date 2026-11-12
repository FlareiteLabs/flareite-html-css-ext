//! Closing tags typed for the user, as VS Code's HTML support does:
//! typing the `>` of a start tag such as `<div class="x">` adds `</div>`
//! after the cursor, and typing `</` finishes the innermost open element's
//! end tag (`</div>`).
//!
//! The text before the cursor is scanned as HTML is tokenized, closely
//! enough for this: comments, doctypes, quoted attribute values, raw text
//! elements (`<script>`, `<style>`, `<textarea>`, `<title>`), void elements
//! (`<br>`, `<img>`), and self-closing tags (`<x />`). SVG is XML: names
//! keep their case, no element is void or raw text, and CDATA sections are
//! text.

/// The rules a document's tags follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
  Html,
  /// XML, such as SVG.
  Xml,
}

/// Elements that have no end tag.
const VOID: &[&str] = &[
  "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
  "track", "wbr",
];

/// Elements whose content is text up to their end tag.
const RAW_TEXT: &[&str] = &["script", "style", "textarea", "title"];

fn is_void(name: &str) -> bool {
  VOID.iter().any(|void| void.eq_ignore_ascii_case(name))
}

fn is_raw_text(name: &str) -> bool {
  RAW_TEXT.iter().any(|raw| raw.eq_ignore_ascii_case(name))
}

fn is_name_char(c: u8) -> bool {
  c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b':' | b'.')
}

/// What to insert after `trigger` was typed at the end of `before` (the
/// document's text up to the cursor): the text, and where in it to leave
/// the cursor, in bytes. `None` for nothing.
pub fn auto_insert(before: &str, trigger: char, dialect: Dialect) -> Option<(String, usize)> {
  let scan = scan(before, dialect);
  match trigger {
    '>' => {
      let name = scan.start_tag_ended_at_end?;
      let text = format!("</{name}>");
      Some((text, 0))
    }
    '/' if before.ends_with("</") && scan.end_tag_opened_at_end => {
      let name = scan.open.last()?;
      let text = format!("{name}>");
      let cursor = text.len();
      Some((text, cursor))
    }
    _ => None,
  }
}

/// What scanning the text before the cursor found.
#[derive(Debug, Default)]
struct Scan {
  /// The elements open at the end, outermost first.
  open: Vec<String>,
  /// The name of a start tag whose `>` ends the text, if it needs an end
  /// tag.
  start_tag_ended_at_end: Option<String>,
  /// Whether the text ends with `</` that begins an end tag (not in a
  /// comment or an attribute value).
  end_tag_opened_at_end: bool,
}

fn scan(text: &str, dialect: Dialect) -> Scan {
  let html = dialect == Dialect::Html;
  let same_name = |a: &str, b: &str| {
    if html {
      a.eq_ignore_ascii_case(b)
    } else {
      a == b
    }
  };
  let bytes = text.as_bytes();
  let mut scan = Scan::default();
  let mut at = 0;
  while let Some(found) = text[at..].find('<') {
    let start = at + found;
    let rest = &text[start..];
    if rest.starts_with("<!--") {
      match text[start + 4..].find("-->") {
        Some(end) => at = start + 4 + end + 3,
        // In a comment to the end.
        None => return Scan::default_with(scan.open),
      }
    } else if rest.starts_with("<![CDATA[") {
      match text[start + 9..].find("]]>") {
        Some(end) => at = start + 9 + end + 3,
        None => return Scan::default_with(scan.open),
      }
    } else if rest.starts_with("</") {
      let name_start = start + 2;
      let mut name_end = name_start;
      while name_end < bytes.len() && is_name_char(bytes[name_end]) {
        name_end += 1;
      }
      if name_end == bytes.len() {
        scan.end_tag_opened_at_end = name_end == name_start;
        scan.start_tag_ended_at_end = None;
        return scan;
      }
      let name = &text[name_start..name_end];
      if let Some(index) = scan.open.iter().rposition(|open| same_name(open, name)) {
        scan.open.truncate(index);
      }
      match text[name_end..].find('>') {
        Some(end) => at = name_end + end + 1,
        None => return Scan::default_with(scan.open),
      }
    } else if rest.starts_with("<!") || rest.starts_with("<?") {
      match text[start..].find('>') {
        Some(end) => at = start + end + 1,
        None => return Scan::default_with(scan.open),
      }
    } else if bytes.get(start + 1).is_some_and(u8::is_ascii_alphabetic) {
      let name_start = start + 1;
      let mut name_end = name_start;
      while name_end < bytes.len() && is_name_char(bytes[name_end]) {
        name_end += 1;
      }
      let name = &text[name_start..name_end];
      // The attributes, up to the tag's `>`, past quoted values.
      let mut i = name_end;
      let mut quote = None;
      let mut closed = None;
      while i < bytes.len() {
        let c = bytes[i];
        match quote {
          Some(q) if c == q => quote = None,
          Some(_) => {}
          None if c == b'"' || c == b'\'' => quote = Some(c),
          None if c == b'>' => {
            closed = Some(i);
            break;
          }
          None => {}
        }
        i += 1;
      }
      let Some(end) = closed else {
        // Still in the tag.
        return Scan::default_with(scan.open);
      };
      let self_closing = end > name_end && bytes[end - 1] == b'/';
      at = end + 1;
      let needs_end = !self_closing && !(html && is_void(name));
      if at == bytes.len() {
        if needs_end {
          scan.start_tag_ended_at_end = Some(name.to_owned());
          scan.open.push(name.to_owned());
        }
        return scan;
      }
      if !needs_end {
        continue;
      }
      if html && is_raw_text(name) {
        // Text up to `</name`, or to the end.
        let end_tag = format!("</{}", name.to_ascii_lowercase());
        match text[at..].to_ascii_lowercase().find(&end_tag) {
          Some(found) => {
            let close = at + found + end_tag.len();
            match text[close..].find('>') {
              Some(end) => at = close + end + 1,
              None => return Scan::default_with(scan.open),
            }
          }
          None => {
            scan.open.push(name.to_owned());
            scan.end_tag_opened_at_end = text.ends_with("</");
            return scan;
          }
        }
      } else {
        scan.open.push(name.to_owned());
      }
    } else {
      at = start + 1;
    }
  }
  scan
}

impl Scan {
  /// Nothing to insert, with these elements open.
  fn default_with(open: Vec<String>) -> Self {
    Self {
      open,
      ..Self::default()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn after_gt_in(before: &str, dialect: Dialect) -> Option<String> {
    auto_insert(before, '>', dialect).map(|(text, cursor)| {
      assert_eq!(cursor, 0);
      text
    })
  }

  fn after_slash_in(before: &str, dialect: Dialect) -> Option<String> {
    auto_insert(before, '/', dialect).map(|(text, cursor)| {
      assert_eq!(cursor, text.len());
      text
    })
  }

  fn after_gt(before: &str) -> Option<String> {
    after_gt_in(before, Dialect::Html)
  }

  fn after_slash(before: &str) -> Option<String> {
    after_slash_in(before, Dialect::Html)
  }

  #[test]
  fn closes_svg_tags_as_xml() {
    let gt = |before| after_gt_in(before, Dialect::Xml);
    let slash = |before| after_slash_in(before, Dialect::Xml);
    assert_eq!(gt("<svg><g>").as_deref(), Some("</g>"));
    // Nothing is void in XML, and names keep their case.
    assert_eq!(gt("<svg><link>").as_deref(), Some("</link>"));
    assert_eq!(gt("<linearGradient>").as_deref(), Some("</linearGradient>"));
    assert_eq!(gt("<svg><path d=\"M0 0\"/>"), None);
    assert_eq!(gt("<?xml version=\"1.0\"?>"), None);
    // A script's content isn't raw text, and CDATA sections are text.
    assert_eq!(gt("<script><![CDATA[ if (a <b>"), None);
    assert_eq!(
      gt("<script><![CDATA[ x ]]></script><g>").as_deref(),
      Some("</g>")
    );
    assert_eq!(slash("<svg><G></g></").as_deref(), Some("G>"));
    assert_eq!(slash("<svg><g></g></").as_deref(), Some("svg>"));
  }

  #[test]
  fn closes_start_tags() {
    assert_eq!(after_gt("<div>").as_deref(), Some("</div>"));
    assert_eq!(after_gt("<p>\n  <span>").as_deref(), Some("</span>"));
    assert_eq!(
      after_gt("<a href=\"x>y\" title='a>b'>").as_deref(),
      Some("</a>")
    );
    assert_eq!(after_gt("<my-element>").as_deref(), Some("</my-element>"));
    assert_eq!(after_gt("<DIV>").as_deref(), Some("</DIV>"));
    assert_eq!(after_gt("<script>").as_deref(), Some("</script>"));
  }

  #[test]
  fn leaves_tags_without_end_tags() {
    assert_eq!(after_gt("<br>"), None);
    assert_eq!(after_gt("<img src=\"a.png\">"), None);
    assert_eq!(after_gt("<x />"), None);
    assert_eq!(after_gt("<div></div>"), None);
    assert_eq!(after_gt("a > b"), None);
    assert_eq!(after_gt("<!DOCTYPE html>"), None);
  }

  #[test]
  fn leaves_comments_attributes_and_raw_text() {
    assert_eq!(after_gt("<!-- <div>"), None);
    assert_eq!(after_gt("<a title=\"<b>"), None);
    assert_eq!(after_gt("<script>if (a <b>"), None);
    assert_eq!(after_gt("<script>x</script><b>").as_deref(), Some("</b>"));
  }

  #[test]
  fn finishes_end_tags() {
    assert_eq!(after_slash("<div>\n  </").as_deref(), Some("div>"));
    assert_eq!(after_slash("<ul><li>a</li></").as_deref(), Some("ul>"));
    assert_eq!(after_slash("<div><br></").as_deref(), Some("div>"));
    assert_eq!(
      after_slash("<script>let a = 1 </").as_deref(),
      Some("script>")
    );
    assert_eq!(after_slash("</"), None);
    assert_eq!(after_slash("<div></div></"), None);
    assert_eq!(after_slash("<div><!-- </"), None);
    assert_eq!(after_slash("<div> a/"), None);
  }
}
