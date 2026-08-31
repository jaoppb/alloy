// The smallest possible muscle script: compute a value and hand it back.
// `alloy --script scripts/hello.rhai` compiles this with RhaiEngine, runs it
// under the execution-limit sandbox, and prints the result.

let greeting = "hello from the muscle";
let answer = 6 * 7;

`${greeting} — the answer is ${answer}`
