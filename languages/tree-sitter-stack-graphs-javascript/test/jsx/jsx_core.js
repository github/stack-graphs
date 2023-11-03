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

const el = <foo bar={x}>{x}</foo>;
//                   ^ defined: 11
//                       ^ defined: 11

const el2 = <x></x>
//           ^ defined: 11
//               ^ defined: 11

// Flow Out

const el = <foo bar={y = 1}>
    {z = 3}
</foo>;

/**/ y;
//   ^ defined: 25

/**/ z;
//   ^ defined: 26

// Flow Across

const el = <foo bar={w = 1}>
    {w}</foo>;
//   ^ defined: 37

const el = <foo bar={q = 1}
    baz={q}></foo>;
//       ^ defined: 41

// Flow Around

/**/ x;
//   ^ defined: 11