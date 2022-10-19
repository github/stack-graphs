type T = number;

// export

// import

debugger;

1;

var a:T;
//    ^ defined: 1
var b:T = (a as T);
//    ^ defined: 1
//         ^ defined: 11
//              ^ defined: 1
var c:T, d:T = (a as T), e:T;
//    ^ defined: 1
//         ^ defined: 1
//              ^ defined: 11
//                   ^ defined: 1
//                         ^ defined: 1
let f:T = (a as T);
//    ^ defined: 1
//         ^ defined: 11
//              ^ defined: 1
const g:T = (a as T);
//      ^ defined: 1
//           ^ defined: 11
//                ^ defined: 1

function foo(x:T) { return (x as T); }
//             ^ defined: 1
//                          ^ defined: 32
//                               ^ defined: 1

function* bar(x:T) { yield (x as T); }
//              ^ defined: 1
//                          ^ defined: 37
//                               ^ defined: 1

interface I { x:T; }
//              ^ defined: 1
interface J extends I { y:T; }
//                  ^ defined: 42
//                        ^ defined: 1

class X { x:T; }
//          ^ defined: 1
class XX implements I { x:T; }
//                  ^ defined: 42
//                        ^ defined: 1
class Y extends X { y:T; }
//              ^ defined: 48
//                    ^ defined: 1
class YY extends XX implements J { y:T; }
//               ^ defined: 50
//                             ^ defined: 44
//                                   ^ defined: 1

abstract class A { x:T; }
//                   ^ defined: 1
abstract class B extends A {}
//                       ^ defined: 61

{}
{
    let x:T = 1;
//        ^ defined: 1
}
{
    let x:T = 1;
//        ^ defined: 1
    let y:T = x;
//        ^ defined: 1
    let z:T = x + y;
//        ^ defined: 1
}

if (true) { let x:T = 1; };
//                ^ defined: 1
if (true) { let x:T = 1; } else { let x:T = 2; };
//                ^ defined: 1
//                                      ^ defined: 1

switch (21*2) {
}
switch (21*2) {
    case 0: { let x:U = 1; } break;
//                  ^ defined: 91
    case 1: type U = T;
//                   ^ defined: 1
    case 2: { let x:T = 1; } break;
//                  ^ defined: 1
    default: { let x:U = 1; } break;
//                   ^ defined: 91
}
let y: U = 1; // tsc: Cannot find name 'U'
//     ^ defined:

for (var i: T = 0; i < 42; i++) { let x: T = 1; }
//          ^ defined: 1
//                 ^ defined: 101
//                         ^ defined: 101
//                                       ^ defined: 1

for (var q in [1, 2, 3]) { let x: T = 1; }
//                                ^ defined: 1

while (1 < 2) { let x: T = 1; }
//                     ^ defined: 1

do { let x: T = 1; } while (1 < 2)
//          ^ defined: 1

try { let x: T = 1; } catch(ex) { let x: T = 1; }
//           ^ defined: 1
//                                       ^ defined: 1
try { let x: T = 1; } catch(ex: any) { let x: T = 1; }
//           ^ defined: 1
//                                            ^ defined: 1
try { let x: T = 1; } finally { let x: T = 1; }
//           ^ defined: 1
//                                     ^ defined: 1
try { let x: T = 1; } catch(ex) { let x: T = 1; } finally { let x: T = 1; }
//           ^ defined: 1
//                                       ^ defined: 1
//                                                                 ^ defined: 1

with({}) { let x: T = 1; };
//                ^ defined: 1

while(true) { break; let x: T = 1; };
//                          ^ defined: 1

while(true) { continue; let x: T = 1; };
//                             ^ defined: 1

function baz() { return; let x: T = 1; }
//                              ^ defined: 1

function baq() { throw {}; let x: T = 1; }
//                                ^ defined: 1

;

lbl: let o: T = 1;
//          ^ defined: 1

enum E {
  C1 = 1,
  C2 = (a as T),
  //    ^ defined: 11
  //         ^ defined: 1
}

let h:E = true ? E.C2 : E.C1;
//    ^ defined: 150
//               ^ defined: 150
//                 ^ defined: 152
//                      ^ defined: 150
//                        ^ defined: 151

type TT = T;
//        ^ defined: 1

// check that definition of `T` flows through all above statements
let fin: T;
//       ^ defined: 1

export {};
