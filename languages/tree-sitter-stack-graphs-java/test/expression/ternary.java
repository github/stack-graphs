class A {
  boolean x;
  void f() {
      x ? x : x;
    //^ defined: 2
    //    ^ defined: 2
    //        ^ defined: 2
  }
}
