class A {
  void f() {
    foo: for(;;) {
      for(;;) {
        break foo;
        //    ^ defined: 3
      }
    }
  }
}
