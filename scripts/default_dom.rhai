// Embedded fallback (C-09, PRD-003:68): when a muscle script fails, the host
// runs this on a clean DomTree in a fresh guarded context. It must stay
// trivial — if it fails too, the Rust `minimal_document()` routine takes over.

let html = document.create_element("html");
document.append_child(html);

let body = html.create_element("body");
html.append_child(body);
