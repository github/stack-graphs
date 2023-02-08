class A {
  int f(int x, boolean y) {
    do return x; while(y);
    //        ^ defined: 2
    //                 ^ defined: 2
  }
}
