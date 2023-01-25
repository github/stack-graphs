class A {
  void f() {
    A a;
    try (A a = f(); a; a.x) {
      //       ^ defined: 2
      //            ^ defined: 3
      //               ^ defined: 3
        f(a);
      //^ defined: 2
      //  ^ defined: 4
    } catch (A e) {
        e;
      //^ defined: 11
        f();
      //^ defined: 2
    } finally {
        f();
      //^ defined: 2
    }
  }
}
