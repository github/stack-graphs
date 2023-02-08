class Foo {
  String bar;

  void setBar(String b) {
    bar = b;
  }
}

class Baz {
  public static void main(String[] args) {
    Foo f = new Foo();
    //^ defined: 1
    //          ^ defined: 1
    f.setBar("high");
 // ^ defined: 11
 //    ^ defined: 4
  }
}
