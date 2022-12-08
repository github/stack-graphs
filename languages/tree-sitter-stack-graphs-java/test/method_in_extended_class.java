class TestClass extends SecondTestClass {
                        // ^ defined: 14
  public static void main(String[] args){
    foo();
 // ^ defined: 10
    bar();
//  ^ defined: 15
  }

  public void foo() {
  }
}

class SecondTestClass {
  public void bar() {
    System.out.println("Hello");
    foo();
//  ^ defined: 21
  }

  public void foo() {
  }
}
