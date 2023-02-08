class A {
  A[] f() {
    for (A x : f()) {
      // ^ defined: 1
      //       ^ defined: 2
      return x;
      //     ^ defined: 3
    }
  }
}
