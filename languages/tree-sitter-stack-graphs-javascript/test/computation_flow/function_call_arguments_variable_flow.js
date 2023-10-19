let obj = {
    x: 1
};



// function declaration
function func_1() {
    return arguments;
}

func_1(obj)[0].x;
//             ^ defined: 2



// generator function declaration
function* gen_func_1() {
    yield arguments;
}

gen_func_1(obj)[0].x;
//                 ^ defined: 2



// assigned function
let func_2 = function () {
    return arguments;
};

func_2(obj)[0].x;
//             ^ defined: 2



// assigned generator function
let gen_func_2 = function* () {
    yield arguments;
};

gen_func_2(obj)[0].x;
//                 ^ defined: 2



// assigned single-param expression-body arrow function
let func_3 = o => arguments;

func_3(obj)[0].x;
//             ^ defined:
// arrow functions have no `arguments`!



// assigned multi-param expression-body arrow function
let func_4 = () => arguments;

func_4(obj, 1)[0].x;
//                ^ defined:
// arrow functions have no `arguments`!



// assigned single-param statement-body arrow function
let func_5 = o => { return arguments; };

func_5(obj)[0].x;
//             ^ defined:
// arrow functions have no `arguments`!



// assigned multi-param statement-body arrow function
let func_6 = () => { return arguments; };

func_6(obj, 1)[0].x;
//                ^ defined:
// arrow functions have no `arguments`!