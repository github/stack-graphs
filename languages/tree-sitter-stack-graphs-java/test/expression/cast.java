class A {
  A f() {
    return (A)(f());
    //      ^ defined: 1
    //         ^ defined: 2
  }
}
