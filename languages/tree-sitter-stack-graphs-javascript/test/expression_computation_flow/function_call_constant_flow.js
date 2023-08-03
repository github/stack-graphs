// function declaration
function func_1() {
    return {
        x: 1
    };
}

func_1().x;
//       ^ defined: 4



// generator function declaration
function* gen_func_1() {
    yield {
        x: 1
    };
}

gen_func_1().x;
//           ^ defined: 16



// assigned function
let func_2 = function () {
    return {
        x: 1
    };
}

func_2().x;
//       ^ defined: 28



// assigned generator function
let gen_func_2 = function* () {
    yield {
        x: 1
    };
}
gen_func_2().x;
//           ^ defined: 40



// assigned single-param expression-body arrow function
let func_3 = o => ({
    x: 1
});

func_3(1).x;
//        ^ defined: 50



// assigned multi-param expression-body arrow function
let func_4 = () => ({
    x: 1
});

func_4().x;
//       ^ defined: 60



// assigned single-param statement-body arrow function
let func_5 = o => {
    return {
        x: 1
    };
};

func_5(1).x;
//        ^ defined: 71



// assigned multi-param statement-body arrow function
let func_6 = () => {
    return {
        x: 1
    };
};

func_6().x;
//       ^ defined: 83