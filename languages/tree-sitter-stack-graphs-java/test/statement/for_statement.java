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

  void h() {
    int k = 5;
    for (int i = 0, j = i; i < 123; k++) {
                     // ^ defined: 23
                                 // ^ defined: 22
    }
  }
}
