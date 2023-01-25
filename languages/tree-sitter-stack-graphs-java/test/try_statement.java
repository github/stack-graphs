class A {
  void f() {
    try {
        f();
      //^ defined: 2
    } catch (A e) {
        e;
      //^ defined: 6
        f();
      //^ defined: 2
    } finally {
        f();
      //^ defined: 2
    }
  }
}
