class A {
  void f() {
    boolean x;
    for(x; x; x) {
      //^ defined: 3
      //   ^ defined: 3
      //      ^ defined: 3
        f();
      //^ defined: 2
    }
  }
  void g() {
    for(boolean x; x; x) {
      //           ^ defined: 13
      //              ^ defined: 13
        g();
      //^ defined: 12
    }
  }
}
