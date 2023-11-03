let x = 1;

// Flow Around

const el = <foo.bar.baz></foo.bar.baz>;

/**/ x;
//   ^ defined: 1

// Flow In

let foo = {
    bar: {
        baz: 1
    }
};

const el2 = <foo.bar.baz />;
//           ^ defined: 12
//               ^ defined: 13
//                   ^ defined: 14