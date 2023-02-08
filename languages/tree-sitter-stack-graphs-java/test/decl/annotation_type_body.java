import java.io.File;
@interface FileQualifier {}

class Foo {
  public void main(@FileQualifier File someFile) {
                   // ^ defined: 2
    return;
  }
}
