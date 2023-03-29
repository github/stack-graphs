class TestClass extends SecondTestClass {
  public static void main(String[] args){
    super.bar();
       // ^ defined: 14
    this.bar();
      // ^ defined: 9
  }

  public void bar() {
  }
}

class SecondTestClass {
  public void bar() {
    System.out.println("Hello");
  }
}
