class A {
  void f() {
    foo: for(;;) {
      for(;;) {
        continue foo;
        //       ^ defined: 3
      }
    }
  }
}
