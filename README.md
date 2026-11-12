# HTML & CSS

HTML, CSS, and SVG for Flareite: highlighting from Tree-sitter grammars (with CSS and JavaScript
inside HTML and SVG), the HTML and CSS language servers, a live preview of HTML files, and HTML
and SVG tags closed as you type them.

## Closing tags

In HTML, typing the `>` of a start tag adds its end tag after the cursor: `<div class="box">`
gets `</div>`. Typing `</` finishes the innermost open element's end tag. Void elements (`<br>`,
`<img>`) and self-closing tags (`<x />`) get none. Nothing is added inside comments or attribute
values, and in a `<script>` or `<style>` only its own end tag is. SVG is closed as XML is: names
keep their case, and no element is void. Undo takes back just the added tag. Reading the text
before the cursor is why the extension asks to read documents.

## Preview and page

Two buttons on an HTML file's tab (or **View: Reopen Editor With…**) show it as a web page:

- **HTML Preview** shows the page beside the file's editor, in the same tab.
- **HTML Page** shows the page by itself, in a tab of its own. Open the file as text in another
  group (**View: Reopen Editor With…**, **Text Editor**, or open it again) to edit it beside it.

The page shows again as the file is edited, in any tab, scrolled to where it was, and when files
it uses (style sheets, scripts, pictures) change. The file button on the tab's right goes back
to the text editor.

- The page is served from the open folder, with the editor's unsaved changes. Use relative
  addresses (`style.css`, `img/logo.png`); addresses from the folder's root (`/style.css`) don't
  resolve.
- Its scripts run, sandboxed: they can't reach Flareite or your files beyond the folder, and the
  page can't reach the network.
- **Making a page for an extension's webview?** A page that links Flareite's kit
  (`<link rel="stylesheet" href="__flareite/kit.css">`) gets the kit, the theme's `--flareite-*`
  variables, and the webview defaults, and follows theme changes, so it looks as it will in the
  extension. It also gets a stand-in `flareite` object, so its scripts run: storage is kept
  while Flareite is open (not the extension's), messages to the extension are logged to the
  console, and `flareite.standIn.receive(message)` delivers one as if the extension sent it. See
  Flareite's `docs/extensions.md` (File pages). Other pages look as they would in a browser.
- Pages show only in a folder you trust, and only files inside it.

## Language servers

`npm install -g vscode-langservers-extracted` provides `vscode-html-language-server` and
`vscode-css-language-server`. Flareite asks before starting them, and shows "not installed" in the
status bar if they're missing; highlighting works either way.

## Building

The preview and closing tags are a small Rust extension (`src/lib.rs`, `src/tags.rs`), built
against the extension API in the editor's repository, checked out beside this one as `editor`:

```sh
cargo build --release
cargo test --target x86_64-pc-windows-msvc   # or your platform's target: the tests run natively
```
