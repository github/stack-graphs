class TestClass {
  public static void test() {
    int x = 5;
    x;
 // ^ defined: 3
    y;
//  ^ defined:
  }

  public static void foo() {
    int y = 4;
    int x = 5;
    y;
//  ^ defined: 11
    x;
//  ^ defined: 12
  }

  public static void invalid() {
    z;
//  ^ defined:
    int z = 8;
  }
}

class Shadowing {
  int x;
  void f() {
      x;
    //^ defined: 27
    int x;
      x;
    //^ defined: 31
  }
}
