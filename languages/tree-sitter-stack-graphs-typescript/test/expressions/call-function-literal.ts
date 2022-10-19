type V = { value: number; }

(function foo(x:V): V {
//                  ^ defined: 1
    return null;
})(null).value;
//       ^ defined: 1

export {};
