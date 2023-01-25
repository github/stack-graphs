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
}
