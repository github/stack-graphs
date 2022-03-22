# -*- python -*-
#
# A Starlark function that visits a Tree Sitter syntax tree
# and prints the information needed to construct the Stack Graph.
# (The SG strategy and node/edge operators aren't implemented yet.)

# All the functions correspond to productions in the Go TS grammar:
# https://github.com/tree-sitter/tree-sitter-go/blob/master/grammar.js

# Note that .__children__ returns a list of all named and unnamed child nodes,
# whereas .foo returns only the first child node named foo, or None if there is none.
# There is no way to request all the nodes with a given name, but there doesn't seem
# to be much need (other than in parameter_or_field_declaration).

# Q. rejecting e.g. "," or "..." feels fragile because it'll
# crash if there's a comment or some other rare whitespace.
# Positive checks are safer but we can't easily check for "is expression".
#
# Q. TS grammars are kinda a fragile public API for us to be building on.
# Any bugfix to the grammar (and there are many bugs) will break our tests (at best).
# e.g. struct field names are a mess, as are paramater declarations (and the two are not unified);
# receive is a "statement" (and is not hooked in to the grammar---I think it will misparse)

# TODO: generics grammar extensions
# TODO: ignore comments systematically

def source_file(f):
  for decl in f.__children__:
    t = decl.__type__  
    if t == "import_declaration":
      import_spec_list = decl.__children__[1]
      for spec in import_spec_list.__children__[1:]:
        if spec.__type__ == "import_spec":
          print("%s: import %s" % (spec.__location__, spec.path.__text__ + (" as " + spec.name.__text__ if spec.name else "")))
    elif t == "function_declaration" or t == "method_declaration":
      func_decl(decl)
    elif t == "package_clause":
      id = decl.__children__[1]
      print("%s: package %s" % (id.__location__, id.__text__))
    else:
      declaration_maybe(decl) # may fail to match (e.g. if decl is a comment)

def func_decl(decl):
  # TODO: declare parameters inside body, but evaluate types outside it.
  # For method, declare receiver inside body, but add to type not lexical env.
  # We'll need to thread SG env nodes down the traversal.
  if decl.receiver: # method?
    parameter_list_or_simple_type(decl.receiver)
  define(decl.name)
  parameter_list_or_simple_type(decl.parameters)
  if decl.result:
    parameter_list_or_simple_type(decl.result)
  if decl.body:
    block(decl.body)

def parameter_list_or_simple_type(params):
  if params.__type__ == "parameter_list":
    for decl in params.__children__[1:-1]: # remove '(' ')'
      if decl.__type__ == "parameter_declaration" or decl.__type__ == "variadic_parameter_declaration":
        parameter_or_field_declaration(decl)
  else:
    expression(params) # simple_type  

def parameter_or_field_declaration(decl):
  if decl.__type__ in "\n;" or decl.__type__ == "comment": # ignore terminators and comments
    return
  # TODO: need to access all 'named' children of decl, not just first.
  for x in decl.__children__[:-1]:
    if x.__type__ != ",":
      define(x)
  expression(decl.type)

def declaration_maybe(decl): # var/const/type
  t = decl.__type__
  if t == "var_declaration" or t == "const_declaration":
    for spec in decl.__children__[1:]:
      if spec.__type__ == "var_spec" or spec.__type__ == "const_spec":
        define(spec.name)
        if spec.type:
          expression(spec.type)
        if spec.value: # (only optional for vars)
          expression_list(spec.value)
  elif t == "type_declaration":
    for spec in decl.__children__[1:]:
      if spec.__type__ == "type_spec" or spec.__type__ == "type_alias":
        define(spec.name)
        expression(spec.type)
  else:
    return False
  return True

def expression(e):
  t = e.__type__
  if t == "identifier" or t == "type_identifier" or t == "package_identifier" or t == "field_identifier":
    use(e.__location__, t, e.__text__)
  elif t == "selector_expression":
    expression(e.operand)
    selector(e.field)
  elif t == "qualified_type":
    expression(e.package)
    selector(e.name)
  elif t == "nil" or t == "true" or t == "false":
    pass
  elif t == "parenthesized_expression":
    expression(e.__children__[1])
  elif t == "binary_expression":
    expression(e.left)
    expression(e.right)
  elif t == "unary_expression":
    expression(e.operand)
  elif t == "interpreted_string_literal" or t == "int_literal":
    pass
  elif t == "expression_list":
    fail(e)
  elif t == "call_expression":
    expression(e.function)
    for arg in e.arguments.__children__[1:-1]: # remove '(' ')'
      if arg.__type__ not in ",...":
        expression(arg)
  elif t == "composite_literal":
    expression(e.type)
    expression(e.body)
  elif t == "literal_value": # only within composite_literal
    for elem in e.__children__[1:-1]: # remove '{' '}'
      if elem.__type__ != ",":
        expression(elem)
  elif t == "keyed_element": # only within composite_literal
    expression(e.__children__[0])
    expression(e.__children__[2])
  elif t == "element": # only within composite_literal
    expression(e.__children__[0])
  elif t == "index_expression":
    expression(e.operand)
    expression(e.index)
  elif t == "type_assertion_expression":
    expression(e.operand)
    expression(e.type)
  elif t == "func_literal":
    parameter_list_or_simple_type(e.parameters)
    if e.result:
      parameter_list_or_simple_type(e.result)
    block(e.body)
  elif t == "pointer_type":
    expression(e.__children__[1])
  elif t == "array_type":
    expression(e.length)
    expression(e.element)
  elif t == "implicit_length_array_type" or t == "slice_type":
    expression(e.element)
  elif t == "channel_type":
    expression(e.value)
  elif t == "function_type":
    parameter_list_or_simple_type(e.parameters)
    if e.result:
      parameter_list_or_simple_type(e.result)
  elif t == "map_type":
    expression(e.key)
    expression(e.value)
  elif t == "struct_type":
    for decl in e.__children__[1].__children__[1:-1]: # remove "{" "}"
      parameter_or_field_declaration(decl)
  elif t == "interface_type":
    specs = e.__children__[1]
    for spec in specs.__children__[1:-1]: # remove "{" "}"
      if spec.__type__ == "method_spec":
        print("interface method", spec.name.__text__)
        parameter_list_or_simple_type(spec.parameters)
        if spec.result:
          parameter_list_or_simple_type(spec.result)
      elif spec.__type__ == "type_identifier":
        print("interface embedding", spec.__text__)
        expression(spec)
      elif spec.__type__ not in "\n;": # ignore terminators
        fail(spec.__location__, spec.__type__)
  else:
    fail("unhandled expr", e, e.__type__, e.__text__, e.__location__)
  
def expression_list(list):
  for e in list.__children__:
    if e.__type__ != ",":
      expression(e)
  
def statement(s):
  t = s.__type__ 
  if t == "return_statement":
    expression_list(s.__children__[1])
  elif t == "go_statement" or t == "defer_statement":
    expression(s.__children__[1])
  elif t == "comment":
    pass
  elif t == "block":
    block(s)
  elif t == "goto_statement" or t == "break_statement" or t == "continue_statement":
    label_name = s.__children__[1]
    use(label_name.__location__, "label", label_name.__text__)
  elif t == "labeled_statement":
    define(s.label)
    if len(s.__children__) > 2:
      statement(s.__children__[2])
  elif t == "fallthrough_statement":
    pass
  elif t == "if_statement":
    # FIXME graph experiments
    # n = node()
    # n.syntax = s
    # print(n.syntax)
    # e = edge(n, n)
    # e.foo = 1 
    # e.foo = 1 
    
    enter(s.__location__, "implicit if-block") # location is approximate
    if s.initializer:
      simple_statement(s.initializer)
    expression(s.condition)
    statement(s.consequence)
    if s.alternative:
      statement(s.alternative)
    leave(s.__location__, "implicit if-block")
  elif t == "for_statement":
    enter(s.__location__, "implicit for-block")
    clause = s.__children__[1]
    if clause.__type__ == "range_clause": # for x = range y
      if clause.left:
        expression_list(clause.left) # TODO: handle := vs =
      expression(clause.right)
    elif clause.__type__ == "for_clause": # for _; _; _
      if clause.initializer:
        simple_statement(clause.initializer)
      if clause.condition:
        expression(clause.condition)
      if clause.update:
        simple_statement(clause.update)
    else:
      expression(clause) # for {}
    block(s.body)
    leave(s.__location__, "implicit for-block") # TODO use loc.right
  elif t == "expression_switch_statement":
    enter(s.__location__, "implicit switch-block")  
    if s.initializer:
      statement(s.initializer)
    if s.value:
      expression(s.value)
    # Find children between { and }. 
    # TODO: This is ugly. node.next_sibling would help.
    on = False
    for elem in s.__children__:
      if on and s.__type__ != "}":
        print("AA", elem.__type__)        
      if elem.type == "{":
        on = True
    leave(s.__location__, "implicit for-block") # TODO use loc.right
   
    # TODO $.type_switch_statement, $.select_statement, $.empty_statement

  elif not declaration_maybe(s):
    simple_statement(s)

def simple_statement(s):
  t = s.__type__ 
  if t == "assignment_statement":
    expression_list(s.left)
    expression_list(s.right)
  elif t == "short_var_declaration":
    # In reality, not all left-side vars are declarations. TODO: express this in SG.
    for id in s.left.__children__:
      if id.__type__ == "identifier": # discard commas
        define(id)
    expression_list(s.right)
  elif t == "inc_statement" or t == "dec_statement":
    expression(s.__children__[0])
  elif t == "send_statement":
    expression(s.channel)
    expression(s.value)
  else:
    expression(s)

def block(b):
  enter(b.__location__, "block")
  for s in b.__children__[1:-1]: # remove '{' '}'
    if s.__type__ not in "\n;": # ignore terminators
      statement(s)
  # TODO: empty_labeled_statement
  leave(b.__location__, "block")  

# Events that should cause Stack Graph creation:

def define(id):
  "Report a definition of a name."
  print("%s:\tdef %s '%s'" % (id.__location__, id.__type__, id.__text__))

def use(loc, type, name):
  "Report a use of a declared name."
  print("%s:\tuse %s '%s'" % (loc, type, name))

def selector(id):
  "Report a .x selector operation."
  print("%s:\tselector .%s" % (id.__location__, id.__text__))

def enter(loc, comment):
  "Reports entry of a lexical block."
  print("%s:\tenter %s" % (loc, comment))

def leave(loc, comment):
  "Reports leaving of a lexical block."
  print("%s:\tleave %s" % (loc, comment))

main = source_file
