interface V { v: number; }

declare let x: Promise<V>;
//                     ^ defined: 1

(await x).v;
//     ^ defined: 3
//        ^ defined: 1

export {};
