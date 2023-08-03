let obj = {
    x: 1
};



// function declaration
function func_1() {
    return obj;
}

func_1().x;
//       ^ defined: 2



// generator function declaration
function* gen_func_1() {
    yield obj;
}

gen_func_1().x;
//           ^ defined: 2



// assigned function
let func_2 = function () {
    return obj;
}

func_2().x;
//       ^ defined: 2



// assigned generator function
let gen_func_2 = function* () {
    yield obj;
}
gen_func_2().x;
//           ^ defined: 2



// assigned single-param expression-body arrow function
let func_3 = y => obj;

func_3().x;
//       ^ defined: 2



// assigned multi-param expression-body arrow function
let func_4 = () => obj;

func_4().x;
//       ^ defined: 2



// assigned single-param statement-body arrow function
let func_5 = y => { return obj; };

func_5().x;
//       ^ defined: 2



// assigned multi-param statement-body arrow function
let func_6 = () => { return obj; };

func_6().x;
//       ^ defined: 2