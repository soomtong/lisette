; Keywords
[
  "as"
  "assert"
  "break"
  "const"
  "continue"
  "defer"
  "else"
  "embed"
  "enum"
  "fn"
  "for"
  "if"
  "impl"
  "import"
  "in"
  "interface"
  "let"
  "loop"
  "match"
  "recover"
  "return"
  "select"
  "struct"
  "task"
  "try"
  "type"
  "var"
  "while"
] @keyword

"self" @variable.builtin

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "&&"
  "||"
  "^"
  "&^"
  "<<"
  ">>"
  "|>"
  "!"
  "&"
  "|"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "&^="
  "<<="
  ">>="
  ".."
  "..="
  "?"
  "=>"
  "->"
] @operator

; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ":"
  ";"
] @punctuation.delimiter

"#" @punctuation.special

; Function definitions
(function_item
  name: (identifier) @function)

(function_signature_item
  name: (identifier) @function)

; Method definitions in impl blocks
(impl_item
  body: (declaration_list
    (function_item
      name: (identifier) @function.method)))

; Function calls
(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (field_expression
    field: (field_identifier) @function.method.call))

; Generic function calls
(generic_call_expression
  function: (identifier) @function.call)

(generic_call_expression
  function: (field_expression
    field: (field_identifier) @function.method.call))

; Types
(type_identifier) @type

(generic_type
  type: (type_identifier) @type)

(scoped_type_identifier
  path: (type_identifier) @module
  name: (type_identifier) @type)

(call_expression
  function: (field_expression
    value: (identifier) @module
    field: (field_identifier) @function.call))

(generic_call_expression
  function: (field_expression
    value: (identifier) @module
    field: (field_identifier) @function.call))

(never_type) @type.builtin

; Prelude compound types
((type_identifier) @type.builtin
  (#match? @type.builtin "^(Option|Result|Partial|Slice|Map|Ref|Channel|Sender|Receiver|Range|RangeInclusive|RangeFrom|RangeTo|RangeToInclusive|EnumeratedSlice|Unknown|Never|Comparable|Ordered|VarArgs|PanicValue)$"))

; Primitive types
((type_identifier) @type.builtin
  (#match? @type.builtin "^(int|int8|int16|int32|int64|uint|uint8|uint16|uint32|uint64|uintptr|byte|rune|float32|float64|complex64|complex128|bool|string|error)$"))

; Struct definitions
(struct_item
  name: (type_identifier) @type.definition)

; Enum definitions
(enum_item
  name: (type_identifier) @type.definition)

; Interface definitions
(interface_item
  name: (type_identifier) @type.definition)

; Impl blocks
(impl_item
  type: (type_identifier) @type)

; Type parameters
(type_parameter
  name: (type_identifier) @type.parameter)

; Type aliases
(type_item
  name: (type_identifier) @type.definition)

; Fields
(field_declaration
  name: (field_identifier) @property.definition)

(field_expression
  field: (field_identifier) @property)

(field_initializer
  field: (field_identifier) @property)

(shorthand_field_initializer
  (identifier) @property)

(field_pattern
  name: (shorthand_field_identifier) @property)

(field_pattern
  name: (field_identifier) @property)

; Struct patterns
(struct_pattern
  type: (type_identifier) @type)

; Tuple struct patterns
(tuple_struct_pattern
  type: (identifier) @type)

(tuple_struct_pattern
  type: (scoped_type_identifier
    name: (type_identifier) @type))

; As-binding patterns
(as_binding_pattern
  name: (identifier) @variable)

; Parameters
(parameter
  pattern: (identifier) @variable.parameter)

(self_parameter) @variable.builtin

(closure_parameters
  (identifier) @variable.parameter)

; Let bindings
(let_declaration
  pattern: (identifier) @variable)

; Constants
(const_item
  name: (identifier) @constant)

; Var declarations
(var_declaration
  name: (identifier) @variable)

; Imports
(import_declaration
  path: (string_literal) @string.special)

(import_declaration
  alias: (identifier) @module)

; Attributes
(attribute_item
  (attribute
    name: (identifier) @attribute))

; Enum variants
(enum_variant
  name: (identifier) @constant)

; Match wildcards
((identifier) @variable
  (#eq? @variable "_"))

; Literals
(string_literal) @string
(raw_string_literal) @string
(format_string) @string
(char_literal) @character
(boolean_literal) @boolean
(integer_literal) @number
(float_literal) @number.float
(imaginary_literal) @number
(backtick_literal) @string.special
(escape_sequence) @string.escape

(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

; Comments
(line_comment) @comment
(doc_comment) @comment.documentation

; Visibility
(visibility_modifier) @keyword.modifier

; Mutability
(mutable_specifier) @keyword.modifier

; Directives
(rawgo_directive
  "@rawgo" @keyword)

; Prelude enum variant constructors
((identifier) @constant.builtin
  (#match? @constant.builtin "^(Some|None|Ok|Err|Both)$"))

; Built-in functions
((identifier) @function.builtin
  (#match? @function.builtin "^(panic|assert_type|complex|real|imaginary|min|max|println|print)$"))

; Identifiers that look like constructors (uppercase first letter)
((identifier) @constructor
  (#match? @constructor "^[A-Z]"))
