class Outer
{
  public Outer()
  {
    Inner i = new Inner();
                // ^ defined: 8
  }
  class Inner
  {
    class Inner2
    {
      class Inner3
      {
        public Inner3()
        {
          Inner2 i2 = new Inner2();
                          // ^ defined: 10
        }
      }
    }
  }
}
