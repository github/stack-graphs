enum A {
  X, Y, Z
}

class B {
  void f() {
    f(A.X);
    //^ defined: 1
  }
}
