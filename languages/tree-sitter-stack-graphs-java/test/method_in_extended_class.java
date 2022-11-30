class TestClass extends SecondTestClass {
                        // ^ defined: 16
  public static void main(String[] args){
    foo();
 // ^ defined: 10
    bar();
//  ^ defined: 17
  }

  public void foo() {
    super.bar();
        // ^ defined: 17
  }
}

class SecondTestClass {
  public void bar() {
    System.out.println("Hello");
    foo();
//  ^ defined: 23
  }

  public void foo() {
  }
}
