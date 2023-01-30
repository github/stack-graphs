class A {
  A x;
  void f() {
      x = x;
    //^ defined: 2
    //    ^ defined: 2
      x.x = x;
    //^ defined: 2
    //  ^ defined: 2
    //      ^ defined: 2
  }
}
