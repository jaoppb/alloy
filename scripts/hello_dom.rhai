// v0.2 I1 micro-deliverable: a muscle script builds a DOM tree through the
// global `document` handle, and `alloy --script scripts/hello_dom.rhai` prints
// its serialized HTML.
//
// The context is created with DOM_READ | DOM_MUTATE; every call below is
// capability-checked at the binding (C-06). Drop DOM_MUTATE and the first
// `create_element` returns PermissionDenied (C-07).

let html = document.create_element("html");
document.append_child(html);

let body = html.create_element("body");
html.append_child(body);

let heading = body.create_element("h1");
body.append_child(heading);
heading.set_attribute("class", "greeting");
let heading_text = heading.create_text("Hello from the muscle");
heading.append_child(heading_text);

let paragraph = body.create_element("p");
body.append_child(paragraph);
let paragraph_text = paragraph.create_text("The DOM lives, and scripts can shape it.");
paragraph.append_child(paragraph_text);
