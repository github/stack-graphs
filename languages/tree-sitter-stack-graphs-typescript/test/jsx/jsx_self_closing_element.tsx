// The core of JSX tests here verify the behavior of the following node types:
//   jsx_element
//   jsx_identifier
//   jsx_attribute
//   jsx_expression
//   jsx_opening_element
//   jsx_closing_element
// There is no real way to avoid testing all of these at once,
// and so we don't even try to.

let x = 1;

// Flow In

const el = <foo bar={x} />;
//                   ^ defined: 11

const el2 = <x />
//           ^ defined: 11

// Flow Out

let y = 2;

const el = <foo bar={y = 1} />;
//                   ^ defined: 23

// Flow Across

const el = <foo bar={y = 1}
    baz={y} />;
//       ^ defined: 23

// Flow Around

/**/ x;
//   ^ defined: 11
