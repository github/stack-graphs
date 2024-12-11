let x = 1;

// Flow Around
namespace noo.bar {
	export let baz = 1;
}

const el = <noo.bar.baz></noo.bar.baz>;
//          ^ defined: 4
//              ^ defined: 4
//                  ^ defined: 5
//                        ^ defined: 4
//                            ^ defined: 4
//                                ^ defined: 5

/**/ x;
//   ^ defined: 1

// Flow In

let foo = {
    bar: {
        baz: 1
    }
};

const el2 = <foo.bar.baz />;
//           ^ defined: 21
//               ^ defined: 22
//                   ^ defined: 23
