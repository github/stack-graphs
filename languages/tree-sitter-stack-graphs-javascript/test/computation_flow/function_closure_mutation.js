let x = 1;

function foo() {
    /**/ x;
    //   ^ defined: 1, 38
}

function* foo() {
    /**/ x;
    //   ^ defined: 1, 38
}

function () {
    /**/ x;
    //   ^ defined: 1, 38
};

function* () {
    /**/ x;
    //   ^ defined: 1, 38
};

() => x;
//    ^ defined: 1, 38

() => {
    /**/ x;
    //   ^ defined: 1, 38
};

class C {
    foo() {
        /**/ x;
        //   ^ defined: 1, 38
    }
}

x = 2;

(function bar() {
    /**/ bar;
    //   ^ defined: 40
});