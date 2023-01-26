class A {
  void f() {
    synchronized (f()) {
      //          ^ defined: 2
        f();
      //^ defined: 2
    }
  }
}
