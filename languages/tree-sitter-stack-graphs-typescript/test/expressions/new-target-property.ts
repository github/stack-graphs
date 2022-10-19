// this test has no assertions and only exists to ensure coverage

function Foo() {
  if (!new.target) { throw 'Foo() must be called with new'; }
}
 export {};
