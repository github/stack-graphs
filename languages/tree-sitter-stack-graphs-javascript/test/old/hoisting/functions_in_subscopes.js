/**/ f1;
//   ^ defined:
/**/ f2;
//   ^ defined:
/**/ f3;
//   ^ defined:
/**/ f4;
//   ^ defined:
/**/ f5;
//   ^ defined:
/**/ f6;
//   ^ defined:
/**/ f7;
//   ^ defined:
/**/ f8;
//   ^ defined:
/**/ f9;
//   ^ defined:
/**/ f10;
//   ^ defined:
/**/ f11;
//   ^ defined:
/**/ f12;
//   ^ defined:
/**/ f13;
//   ^ defined:
/**/ f14;
//   ^ defined:
/**/ f15;
//   ^ defined:
/**/ f16;
//   ^ defined:
/**/ f17;
//   ^ defined:

while (x) {
    /**/ f1;
    //   ^ defined: 40

    function f1() { }
}

for (let x = 0; x < 10; x++) {
    /**/ f2;
    //   ^ defined: 47

    function f2() { }
}

if (x) {
    /**/ f3;
    //   ^ defined: 54

    function f3() { }
} else {
    /**/ f4;
    //   ^ defined: 59

    function f4() { }
}

for (x in y) {
    /**/ f5;
    //   ^ defined: 66

    function f5() { }
}

do {
    /**/ f6;
    //   ^ defined: 73

    function f6() { }
} while (x);

try {
    /**/ f7;
    //   ^ defined: 80

    function f7() { }
} catch {
    /**/ f8;
    //   ^ defined: 85

    function f8() { }
} finally {
    /**/ f9;
    //   ^ defined: 90

    function f9() { }
}

with (x) {
    /**/ f10;
    //   ^ defined: 97

    function f10() { }
}

switch (x) {
    case 0:
        /**/ f11;
        //   ^ defined: 105

        function f11() { }
}

{
    /**/ f12;
    //   ^ defined: 112

    function f12() { }
}

function foo() {
    /**/ f13;
    //   ^ defined: 119

    function f13() { }
}

function* foo() {
    /**/ f14;
    //   ^ defined: 126

    function f14() { }
}

(function () {
    /**/ f15;
    //   ^ defined: 133

    function f15() { }
});

(function* () {
    /**/ f16;
    //   ^ defined: 140

    function f16() { }
});

(() => {
    /**/ f17;
    //   ^ defined: 147

    function f17() { }
});

// Some tests of bare single-statement bodies

while (x) function f1() { }

for (let x = 0; x < 10; x++) function f2() { }

if (x) function f3() { }
else function f4() { }

for (x in y) function f5() { }

do function f6() { } while (x);

with (x) function f10() { }
