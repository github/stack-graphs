class A {
  void f() {
    A b;
    try (A a = f(); a; b.x) {
      //       ^ defined: 2
      //            ^ defined: 4
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
