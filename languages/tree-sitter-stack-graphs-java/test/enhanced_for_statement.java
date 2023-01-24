class A {
  int[] f() {
    for (int x : f()) {
      //         ^ defined: 2
      return x;
      //     ^ defined: 3
    }
  }
}
