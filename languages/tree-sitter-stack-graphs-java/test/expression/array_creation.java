class A {
  void f() {
    int x;
    new A[x][x];
    //  ^ defined: 1
    //    ^ defined: 3
    //       ^ defined: 3
    A[] as = { x, x };
    //         ^ defined: 3
    //            ^ defined: 3
    A[] as = new A[x][];
    //           ^ defined: 1
    //             ^ defined: 3
  }
}
