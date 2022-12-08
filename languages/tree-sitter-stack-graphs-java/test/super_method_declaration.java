class TestClass extends SecondTestClass {
  public static void main(String[] args){
    super.bar();
       // ^ defined: 12
  }

  public void bar() {
  }
}

class SecondTestClass {
  public void bar() {
    System.out.println("Hello");
  }
}
