// Importing required classes
import java.lang.annotation.ElementType;
import java.lang.annotation.Target;
 
// Using target annotation to annotate a type
@Target(ElementType.TYPE_USE)
// ^ defined: 3
 
// Declaring a simple type annotation
@interface TypeAnnoDemo{}
 
// Main class
public class GFG {
   
    // Main driver method
    public static void main(String[] args) {
 
        // Annotating the type of a string
        @TypeAnnoDemo String string = "I am annotated with a type annotation";
        // ^ defined: 9
        System.out.println(string);
        abc();
    }
 
    // Annotating return type of a function
    static @TypeAnnoDemo int abc() {
       
        System.out.println("This function's  return type is annotated");
       
        return 0;
    }
}
