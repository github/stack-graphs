class TestClass extends SecondTestClass {
  public static void main(String[] args){
    int foo = 10;
    super.foo;
       // ^ defined: 10
  }
}

class SecondTestClass {
  int foo = 5;
}
