class A {
  boolean f(A x) {
    return x instanceof A;
    //     ^ defined: 2
    //                  ^ defined: 1
  }
}
