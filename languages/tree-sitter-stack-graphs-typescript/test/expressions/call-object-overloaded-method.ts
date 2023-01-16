class Super {
  bar() { }
}

class Sub extends Super {
  main() {
    this.bar();
    //   ^ defined: 13
    super.bar();
    //    ^ defined: 2
  }

  bar() { }
}
