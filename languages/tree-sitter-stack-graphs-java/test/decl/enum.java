enum A {
  X, Y, Z
}

class B {
    A f() {
  //^ defined: 1
    f(X);
    //^ defined: 2
  }
}
