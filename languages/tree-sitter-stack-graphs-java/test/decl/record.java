record A(int x) {
    A a;
  //^ defined: 1
  void f() {
      x;
    //^ defined: 1
  }
}
class B {
    A b;
  //^ defined: 1
}
