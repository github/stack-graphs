interface A {
    f: number;
}

interface B {
    g: number;
}

abstract class C
    implements A, B
//             ^ defined: 1
//                ^ defined: 5
{
    test() {
        return this.f + this.g;
        //          ^ defined: 2
        //                   ^ defined: 6
    }
}
