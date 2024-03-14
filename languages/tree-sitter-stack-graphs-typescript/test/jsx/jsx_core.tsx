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

let y = 0;
let z = 2;

const el = <foo bar={y = 1}>
//                   ^ defined: 23
    {z = 3}
//   ^ defined: 24
</foo>;

/**/ y;
//   ^ defined: 23

/**/ z;
//   ^ defined: 24

/**/ x;
//   ^ defined: 11
