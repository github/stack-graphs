class A {
    @A @A(true) @A(k = true) A x;
  // ^ defined: 1
  //    ^ defined: 1
  //             ^ defined: 1
  //                         ^ defined: 1
}
