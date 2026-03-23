# Chapter 4. Types, Values, and Variables


## Contents

[4.1. The Kinds of Types and Values](ch04-types-values-variables.md#jls-4.1)

[4.2. Primitive Types and Values](ch04-types-values-variables.md#jls-4.2)

[4.2.1. Integral Types and Values](ch04-types-values-variables.md#jls-4.2.1)

[4.2.2. Integer Operations](ch04-types-values-variables.md#jls-4.2.2)

[4.2.3. Floating-Point Types and Values](ch04-types-values-variables.md#jls-4.2.3)

[4.2.4. Floating-Point Operations](ch04-types-values-variables.md#jls-4.2.4)

[4.2.5. The `boolean` Type and boolean Values](ch04-types-values-variables.md#jls-4.2.5)

[4.3. Reference Types and Values](ch04-types-values-variables.md#jls-4.3)

[4.3.1. Objects](ch04-types-values-variables.md#jls-4.3.1)

[4.3.2. The Class `Object`](ch04-types-values-variables.md#jls-4.3.2)

[4.3.3. The Class `String`](ch04-types-values-variables.md#jls-4.3.3)

[4.3.4. When Reference Types Are the Same](ch04-types-values-variables.md#jls-4.3.4)

[4.4. Type Variables](ch04-types-values-variables.md#jls-4.4)

[4.5. Parameterized Types](ch04-types-values-variables.md#jls-4.5)

[4.5.1. Type Arguments of Parameterized Types](ch04-types-values-variables.md#jls-4.5.1)

[4.5.2. Members and Constructors of Parameterized Types](ch04-types-values-variables.md#jls-4.5.2)

[4.6. Type Erasure](ch04-types-values-variables.md#jls-4.6)

[4.7. Reifiable Types](ch04-types-values-variables.md#jls-4.7)

[4.8. Raw Types](ch04-types-values-variables.md#jls-4.8)

[4.9. Intersection Types](ch04-types-values-variables.md#jls-4.9)

[4.10. Subtyping](ch04-types-values-variables.md#jls-4.10)

[4.10.1. Subtyping among Primitive Types](ch04-types-values-variables.md#jls-4.10.1)

[4.10.2. Subtyping among Class and Interface Types](ch04-types-values-variables.md#jls-4.10.2)

[4.10.3. Subtyping among Array Types](ch04-types-values-variables.md#jls-4.10.3)

[4.10.4. Least Upper Bound](ch04-types-values-variables.md#jls-4.10.4)

[4.10.5. Type Projections](ch04-types-values-variables.md#jls-4.10.5)

[4.11. Where Types Are Used](ch04-types-values-variables.md#jls-4.11)

[4.12. Variables](ch04-types-values-variables.md#jls-4.12)

[4.12.1. Variables of Primitive Type](ch04-types-values-variables.md#jls-4.12.1)

[4.12.2. Variables of Reference Type](ch04-types-values-variables.md#jls-4.12.2)

[4.12.3. Kinds of Variables](ch04-types-values-variables.md#jls-4.12.3)

[4.12.4. `final` Variables](ch04-types-values-variables.md#jls-4.12.4)

[4.12.5. Initial Values of Variables](ch04-types-values-variables.md#jls-4.12.5)

[4.12.6. Types, Classes, and Interfaces](ch04-types-values-variables.md#jls-4.12.6)


The Java programming language is a *statically typed* language, which means that every variable and every expression has a type that is known at compile time.

The Java programming language is also a *strongly typed* language, because types limit the values that a variable ([§4.12](ch04-types-values-variables.md#jls-4.12)) can hold or that an expression can produce, limit the operations supported on those values, and determine the meaning of the operations. Strong static typing helps detect errors at compile time.

The types of the Java programming language are divided into two kinds: primitive types and reference types. The primitive types ([§4.2](ch04-types-values-variables.md#jls-4.2)) are the `boolean` type and the numeric types. The numeric types are the integral types `byte`, `short`, `int`, `long`, and `char`, and the floating-point types `float` and `double`. The reference types ([§4.3](ch04-types-values-variables.md#jls-4.3)) are class types, interface types, and array types. There is also a special null type. An object ([§4.3.1](ch04-types-values-variables.md#jls-4.3.1)) is a dynamically created instance of a class type or a dynamically created array. The values of a reference type are references to objects. All objects, including arrays, support the methods of class `Object` ([§4.3.2](ch04-types-values-variables.md#jls-4.3.2)). String literals are represented by `String` objects ([§4.3.3](ch04-types-values-variables.md#jls-4.3.3)).


## 4.1. The Kinds of Types and Values


There are two kinds of types in the Java programming language: primitive types ([§4.2](ch04-types-values-variables.md#jls-4.2)) and reference types ([§4.3](ch04-types-values-variables.md#jls-4.3)). There are, correspondingly, two kinds of data values that can be stored in variables, passed as arguments, returned by methods, and operated on: primitive values ([§4.2](ch04-types-values-variables.md#jls-4.2)) and reference values ([§4.3](ch04-types-values-variables.md#jls-4.3)).


Type:


[PrimitiveType](ch04-types-values-variables.md#jls-PrimitiveType "PrimitiveType")  
[ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType")


There is also a special *null type*, the type of the expression `null` ([§3.10.8](ch03-lexical-structure.md#jls-3.10.8), [§15.8.1](ch15-expressions.md#jls-15.8.1)), which has no name.

Because the null type has no name, it is impossible to declare a variable of the null type or to cast to the null type.

The null reference is the only possible value of an expression of null type.

The null reference can always be assigned or cast to any reference type ([§5.2](ch05-conversions-contexts.md#jls-5.2), [§5.3](ch05-conversions-contexts.md#jls-5.3), [§5.5](ch05-conversions-contexts.md#jls-5.5)).

In practice, the programmer can ignore the null type and just pretend that `null` is merely a special literal that can be of any reference type.


## 4.2. Primitive Types and Values


A primitive type is predefined by the Java programming language and named by its reserved keyword ([§3.9](ch03-lexical-structure.md#jls-3.9)):


PrimitiveType:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [NumericType](ch04-types-values-variables.md#jls-NumericType "NumericType")  
{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `boolean`


NumericType:


[IntegralType](ch04-types-values-variables.md#jls-IntegralType "IntegralType")  
[FloatingPointType](ch04-types-values-variables.md#jls-FloatingPointType "FloatingPointType")


IntegralType:


(one of)  
`byte` `short` `int` `long` `char`


FloatingPointType:


(one of)  
`float` `double`


Primitive values do not share state with other primitive values.

The *numeric types* are the integral types and the floating-point types.

The *integral types* are `byte`, `short`, `int`, and `long`, whose values are 8-bit, 16-bit, 32-bit and 64-bit signed two's-complement integers, respectively, and `char`, whose values are 16-bit unsigned integers representing UTF-16 code units ([§3.1](ch03-lexical-structure.md#jls-3.1)).

The *floating-point types* are `float`, whose values exactly correspond to the 32-bit IEEE 754 binary32 floating-point numbers, and `double`, whose values exactly correspond to the 64-bit IEEE 754 binary64 floating-point numbers.

The `boolean` type has exactly two values: `true` and `false`.


### 4.2.1. Integral Types and Values


The values of the integral types are integers in the following ranges:


- For `byte`, from -128 to 127, inclusive

- For `short`, from -32768 to 32767, inclusive

- For `int`, from -2147483648 to 2147483647, inclusive

- For `long`, from -9223372036854775808 to 9223372036854775807, inclusive

- For `char`, from `'\u0000'` to `'\uffff'` inclusive, that is, from 0 to 65535


### 4.2.2. Integer Operations


The Java programming language provides a number of operators that act on integral values:


- The comparison operators, which result in a value of type `boolean`:


  - The numerical comparison operators `<`, `<=`, `>`, and `>=` ([§15.20.1](ch15-expressions.md#jls-15.20.1))

  - The numerical equality operators `==` and `!=` ([§15.21.1](ch15-expressions.md#jls-15.21.1))

  

- The numerical operators, which result in a value of type `int` or `long`:


  - The unary plus and minus operators `+` and `-` ([§15.15.3](ch15-expressions.md#jls-15.15.3), [§15.15.4](ch15-expressions.md#jls-15.15.4))

  - The multiplicative operators `*`, `/`, and `%` ([§15.17](ch15-expressions.md#jls-15.17))

  - The additive operators `+` and `-` ([§15.18](ch15-expressions.md#jls-15.18))

  - The increment operator `++`, both prefix ([§15.15.1](ch15-expressions.md#jls-15.15.1)) and postfix ([§15.14.2](ch15-expressions.md#jls-15.14.2))

  - The decrement operator `--`, both prefix ([§15.15.2](ch15-expressions.md#jls-15.15.2)) and postfix ([§15.14.3](ch15-expressions.md#jls-15.14.3))

  - The signed and unsigned shift operators `<<`, `>>`, and `>>>` ([§15.19](ch15-expressions.md#jls-15.19))

  - The bitwise complement operator `~` ([§15.15.5](ch15-expressions.md#jls-15.15.5))

  - The integer bitwise operators `&`, `^`, and `|` ([§15.22.1](ch15-expressions.md#jls-15.22.1))

  

- The conditional operator `? :` ([§15.25](ch15-expressions.md#jls-15.25))

- The cast operator ([§15.16](ch15-expressions.md#jls-15.16)), which can convert from an integral value to a value of any specified numeric type

- The string concatenation operator `+` ([§15.18.1](ch15-expressions.md#jls-15.18.1)), which, when given a `String` operand and an integral operand, will convert the integral operand to a `String` (the decimal form of a `byte`, `short`, `int`, or `long` operand, or the character of a `char` operand), and then produce a newly created `String` that is the concatenation of the two strings


Other useful constructors, methods, and constants are predefined in the classes `Byte`, `Short`, `Integer`, `Long`, and `Character`.

If an integer operator other than a shift operator has at least one operand of type `long`, then the operation is carried out using 64-bit precision, and the result of the numerical operator is of type `long`. If the other operand is not `long`, it is first widened ([§5.1.5](ch05-conversions-contexts.md#jls-5.1.5)) to type `long` by numeric promotion ([§5.6](ch05-conversions-contexts.md#jls-5.6)).

Otherwise, the operation is carried out using 32-bit precision, and the result of the numerical operator is of type `int`. If either operand is not an `int`, it is first widened to type `int` by numeric promotion.

The integer operators do not indicate overflow or underflow in any way.

Any value of any integral type may be cast to or from any numeric type. There are no casts between integral types and the type `boolean`.

See [§4.2.5](ch04-types-values-variables.md#jls-4.2.5) for an idiom to convert integer expressions to `boolean`.

An integer operator can throw an exception ([§11 (Exceptions)](ch11-exceptions.md)) for the following reasons:


- Any integer operator can throw a `NullPointerException` if unboxing conversion ([§5.1.8](ch05-conversions-contexts.md#jls-5.1.8)) of a null reference is required.

- The integer divide operator `/` ([§15.17.2](ch15-expressions.md#jls-15.17.2)) and the integer remainder operator `%` ([§15.17.3](ch15-expressions.md#jls-15.17.3)) can throw an `ArithmeticException` if the right-hand operand is zero.

- The increment and decrement operators `++` ([§15.14.2](ch15-expressions.md#jls-15.14.2), [§15.15.1](ch15-expressions.md#jls-15.15.1)) and `--` ([§15.14.3](ch15-expressions.md#jls-15.14.3), [§15.15.2](ch15-expressions.md#jls-15.15.2)) can throw an `OutOfMemoryError` if boxing conversion ([§5.1.7](ch05-conversions-contexts.md#jls-5.1.7)) is required and there is not sufficient memory available to perform the conversion.


**Example 4.2.2-1. Integer Operations**


``` programlisting

class Test {
    public static void main(String[] args) {
        int i = 1000000;
        System.out.println(i * i);
        long l = i;
        System.out.println(l * l);
        System.out.println(20296 / (l - i));
    }
}
```

This program produces the output:

``` screen

-727379968
1000000000000
```

and then encounters an `ArithmeticException` in the division by `l - i`, because `l - i` is zero. The first multiplication is performed in 32-bit precision, whereas the second multiplication is a `long` multiplication. The value `-727379968` is the decimal value of the low 32 bits of the mathematical result, `1000000000000`, which is a value too large for type `int`.


  


### 4.2.3. Floating-Point Types and Values


The floating-point types are `float` and `double`, which are conceptually associated with the 32-bit binary32 and 64-bit binary64 floating-point formats for IEEE 754 values and operations, as specified in the IEEE 754 Standard ([§1.7](ch01-introduction.md#jls-1.7)).

In Java SE 15 and later, the Java programming language uses the 2019 version of the IEEE 754 Standard. Prior to Java SE 15, the Java programming language used the 1985 version of the IEEE 754 Standard, where the binary32 format was known as the single format and the binary64 format was known as the double format.

IEEE 754 includes not only positive and negative numbers that consist of a sign and magnitude, but also positive and negative zeros, positive and negative *infinities*, and special *Not-a-Number* values (hereafter abbreviated NaN). A NaN value is used to represent the result of certain invalid operations such as dividing zero by zero. NaN constants of both `float` and `double` type are predefined as `Float.NaN` and `Double.NaN`.

The finite nonzero values of a floating-point type can all be expressed in the form *s* ⋅ *m* ⋅ 2<sup>(*e* - `N` + 1)</sup>, where:


- *s* is +1 or -1,

- *m* is a positive integer less than 2<sup>`N`</sup>,

- *e* is an integer between *E<sub>min</sub>* = -(2<sup>K-1</sup>-2) and *E<sub>max</sub>* = 2<sup>K-1</sup>-1, inclusive, and

- `N` and K are parameters that depend on the type.


Some values can be represented in this form in more than one way. For example, supposing that a value `v` of a floating-point type might be represented in this form using certain values for *s*, *m*, and *e*, then if it happened that *m* were even and *e* were less than 2<sup>K-1</sup>, one could halve *m* and increase *e* by 1 to produce a second representation for the same value `v`.

A representation in this form is called *normalized* if *m* ≥ 2<sup>`N`-1</sup>; otherwise the representation is said to be *subnormal*. If a value of a floating-point type cannot be represented in such a way that *m* ≥ 2<sup>`N`-1</sup>, then the value is said to be a *subnormal value*, because its magnitude is below the magnitude of the smallest normalized value.

The constraints on the parameters `N` and K (and on the derived parameters *E<sub>min</sub>* and *E<sub>max</sub>*) for `float` and `double` are summarized in [Table 4.2.3-A](ch04-types-values-variables.md#jls-4.2.3-150-A).


**Table 4.2.3-A. Floating-point parameters**


| Parameter                                       | `float` | `double` |
|-------------------------------------------------|---------|----------|
| `N`                                             | 24      | 53       |
| K                     | 8       | 11       |
| *E<sub>max</sub>* | +127    | +1023    |
| *E<sub>min</sub>* | -126    | -1022    |


  

Except for NaN, floating-point values are *ordered*. Arranged from smallest to largest, they are negative infinity, negative finite nonzero values, negative and positive zero, positive finite nonzero values, and positive infinity.

IEEE 754 allows multiple distinct NaN values for each of its binary32 and binary64 floating-point formats. However, the Java SE Platform generally treats NaN values of a given floating-point type as though collapsed into a single canonical value, and hence this specification normally refers to an arbitrary NaN as though to a canonical value.

Under IEEE 754, a floating-point operation with non-NaN arguments may generate a NaN result. IEEE 754 specifies a set of NaN bit patterns, but does not mandate which particular NaN bit pattern is used to represent a NaN result; this is left to the hardware architecture. A programmer can create NaNs with different bit patterns to encode, for example, retrospective diagnostic information. These NaN values can be created with the `Float.intBitsToFloat` and `Double.longBitsToDouble` methods for `float` and `double`, respectively. Conversely, to inspect the bit patterns of NaN values, the `Float.floatToRawIntBits` and `Double.doubleToRawLongBits` methods can be used for `float` and `double`, respectively.

Positive zero and negative zero compare equal, so the result of the expression `0.0==-0.0` is `true` and the result of `0.0>-0.0` is false. Other operations can distinguish positive and negative zero; for example, `1.0/0.0` has the value positive infinity, while the value of `1.0/-0.0` is negative infinity.

NaN is *unordered*, so:


- The numerical comparison operators `<`, `<=`, `>`, and `>=` return `false` if either or both operands are NaN ([§15.20.1](ch15-expressions.md#jls-15.20.1)).

  In particular, `(x<y) == !(x>=y)` will be `false` if `x` or `y` is NaN.

- The equality operator `==` returns `false` if either operand is NaN.

- The inequality operator `!=` returns `true` if either operand is NaN ([§15.21.1](ch15-expressions.md#jls-15.21.1)).

  In particular, `x!=x` is `true` if and only if `x` is NaN.


### 4.2.4. Floating-Point Operations


The Java programming language provides a number of operators that act on floating-point values:


- The comparison operators, which result in a value of type `boolean`:


  - The numerical comparison operators `<`, `<=`, `>`, and `>=` ([§15.20.1](ch15-expressions.md#jls-15.20.1))

  - The numerical equality operators `==` and `!=` ([§15.21.1](ch15-expressions.md#jls-15.21.1))

  

- The numerical operators, which result in a value of type `float` or `double`:


  - The unary plus and minus operators `+` and `-` ([§15.15.3](ch15-expressions.md#jls-15.15.3), [§15.15.4](ch15-expressions.md#jls-15.15.4))

  - The multiplicative operators `*`, `/`, and `%` ([§15.17](ch15-expressions.md#jls-15.17))

  - The additive operators `+` and `-` ([§15.18.2](ch15-expressions.md#jls-15.18.2))

  - The increment operator `++`, both prefix ([§15.15.1](ch15-expressions.md#jls-15.15.1)) and postfix ([§15.14.2](ch15-expressions.md#jls-15.14.2))

  - The decrement operator `--`, both prefix ([§15.15.2](ch15-expressions.md#jls-15.15.2)) and postfix ([§15.14.3](ch15-expressions.md#jls-15.14.3))

  

- The conditional operator `? :` ([§15.25](ch15-expressions.md#jls-15.25))

- The cast operator ([§15.16](ch15-expressions.md#jls-15.16)), which can convert from a floating-point value to a value of any specified numeric type

- The string concatenation operator `+` ([§15.18.1](ch15-expressions.md#jls-15.18.1)), which, when given a `String` operand and a floating-point operand, will convert the floating-point operand to a `String` representing its value in decimal form (without information loss), and then produce a newly created `String` by concatenating the two strings


Other useful constructors, methods, and constants are predefined in the classes `Float`, `Double`, and `Math`.

If at least one of the operands to a binary operator is of floating-point type, then the operation is a floating-point operation, even if the other operand is integral.

If at least one of the operands to a numerical operator is of type `double`, then the operation is carried out using 64-bit floating-point arithmetic, and the result of the numerical operator is a value of type `double`. If the other operand is not a `double`, it is first widened ([§5.1.5](ch05-conversions-contexts.md#jls-5.1.5)) to type `double` by numeric promotion ([§5.6](ch05-conversions-contexts.md#jls-5.6)).

Otherwise, at least one of the operands is of type `float`; the operation is carried out using 32-bit floating-point arithmetic, and the result of the numerical operator is a value of type `float`. If the other operand is not a `float`, it is first widened to type `float` by numeric promotion.

Floating-point arithmetic is carried out in accordance with the rules of the IEEE 754 Standard, including for overflow and underflow ([§15.4](ch15-expressions.md#jls-15.4)), with the exception of the remainder operator `%` ([§15.17.3](ch15-expressions.md#jls-15.17.3)).

Any value of a floating-point type may be cast to or from any numeric type. There are no casts between floating-point types and the type `boolean`.

See [§4.2.5](ch04-types-values-variables.md#jls-4.2.5) for an idiom to convert floating-point expressions to `boolean`.

A floating-point operator can throw an exception ([§11 (Exceptions)](ch11-exceptions.md)) for the following reasons:


- Any floating-point operator can throw a `NullPointerException` if unboxing conversion ([§5.1.8](ch05-conversions-contexts.md#jls-5.1.8)) of a null reference is required.

- The increment and decrement operators `++` ([§15.14.2](ch15-expressions.md#jls-15.14.2), [§15.15.1](ch15-expressions.md#jls-15.15.1)) and `--` ([§15.14.3](ch15-expressions.md#jls-15.14.3), [§15.15.2](ch15-expressions.md#jls-15.15.2)) can throw an `OutOfMemoryError` if boxing conversion ([§5.1.7](ch05-conversions-contexts.md#jls-5.1.7)) is required and there is not sufficient memory available to perform the conversion.


**Example 4.2.4-1. Floating-point Operations**


``` programlisting

class Test {
    public static void main(String[] args) {
        // An example of overflow:
        double d = 1e308;
        System.out.print("overflow produces infinity: ");
        System.out.println(d + "*10==" + d*10);
        // An example of gradual underflow:
        d = 1e-305 * Math.PI;
        System.out.print("gradual underflow: " + d + "\n   ");
        for (int i = 0; i < 4; i++)
            System.out.print(" " + (d /= 100000));
        System.out.println();
        // An example of NaN:
        System.out.print("0.0/0.0 is Not-a-Number: ");
        d = 0.0/0.0;
        System.out.println(d);
        // An example of inexact results and rounding:
        System.out.print("inexact results with float:");
        for (int i = 0; i < 100; i++) {
            float z = 1.0f / i;
            if (z * i != 1.0f)
                System.out.print(" " + i);
        }
        System.out.println();
        // Another example of inexact results and rounding:
        System.out.print("inexact results with double:");
        for (int i = 0; i < 100; i++) {
            double z = 1.0 / i;
            if (z * i != 1.0)
                System.out.print(" " + i);
        }
        System.out.println();
        // An example of cast to integer rounding:
        System.out.print("cast to int rounds toward 0: ");
        d = 12345.6;
        System.out.println((int)d + " " + (int)(-d));
    }
}
```

This program produces the output:

``` screen

overflow produces infinity: 1.0E308*10==Infinity
gradual underflow: 3.141592653589793E-305
    3.1415926535898E-310 3.141592653E-315 3.142E-320 0.0
0.0/0.0 is Not-a-Number: NaN
inexact results with float: 0 41 47 55 61 82 83 94 97
inexact results with double: 0 49 98
cast to int rounds toward 0: 12345 -12345
```

This example demonstrates, among other things, that gradual underflow can result in a gradual loss of precision.

The results when `i` is `0` involve division by zero, so that `z` becomes positive infinity, and `z * 0` is NaN, which is not equal to `1.0`.


  


### 4.2.5. The `boolean` Type and boolean Values


The `boolean` type represents a logical quantity with two possible values, indicated by the literals `true` and `false` ([§3.10.3](ch03-lexical-structure.md#jls-3.10.3)).

The boolean operators are:


- The relational operators `==` and `!=` ([§15.21.2](ch15-expressions.md#jls-15.21.2))

- The logical complement operator `!` ([§15.15.6](ch15-expressions.md#jls-15.15.6))

- The logical operators `&`, `^`, and `|` ([§15.22.2](ch15-expressions.md#jls-15.22.2))

- The conditional-and and conditional-or operators `&&` ([§15.23](ch15-expressions.md#jls-15.23)) and `||` ([§15.24](ch15-expressions.md#jls-15.24))

- The conditional operator `? :` ([§15.25](ch15-expressions.md#jls-15.25))

- The string concatenation operator `+` ([§15.18.1](ch15-expressions.md#jls-15.18.1)), which, when given a `String` operand and a `boolean` operand, will convert the `boolean` operand to a `String` (either `"true"` or `"false"`), and then produce a newly created `String` that is the concatenation of the two strings


Boolean expressions determine the control flow in several kinds of statements:


- The `if` statement ([§14.9](ch14-blocks-statements-patterns.md#jls-14.9))

- The `while` statement ([§14.12](ch14-blocks-statements-patterns.md#jls-14.12))

- The `do` statement ([§14.13](ch14-blocks-statements-patterns.md#jls-14.13))

- The `for` statement ([§14.14](ch14-blocks-statements-patterns.md#jls-14.14))


A `boolean` expression also determines which subexpression is evaluated in the conditional `? :` operator ([§15.25](ch15-expressions.md#jls-15.25)).

Only `boolean` and `Boolean` expressions can be used in control flow statements and as the first operand of the conditional operator `? :`.

An integer or floating-point expression `x` can be converted to a `boolean` value, following the C language convention that any nonzero value is `true`, by the expression `x!=0`.

An object reference `obj` can be converted to a `boolean` value, following the C language convention that any reference other than `null` is `true`, by the expression `obj!=null`.

A `boolean` value can be converted to a `String` by string conversion ([§5.4](ch05-conversions-contexts.md#jls-5.4)).

A `boolean` value may be cast to type `boolean`, `Boolean`, or `Object` ([§5.5](ch05-conversions-contexts.md#jls-5.5)). No other casts on type `boolean` are allowed.


## 4.3. Reference Types and Values


There are four kinds of *reference types*: class types ([§8.1](ch08-classes.md#jls-8.1)), interface types ([§9.1](ch09-interfaces.md#jls-9.1)), type variables ([§4.4](ch04-types-values-variables.md#jls-4.4)), and array types ([§10.1](ch10-arrays.md#jls-10.1)).


ReferenceType:


[ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType")  
[TypeVariable](ch04-types-values-variables.md#jls-TypeVariable "TypeVariable")  
[ArrayType](ch04-types-values-variables.md#jls-ArrayType "ArrayType")


ClassOrInterfaceType:


[ClassType](ch04-types-values-variables.md#jls-ClassType "ClassType")  
[InterfaceType](ch04-types-values-variables.md#jls-InterfaceType "InterfaceType")


ClassType:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\]  
[PackageName](ch06-names.md#jls-PackageName "PackageName") `.` {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\]  
[ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType") `.` {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\]


InterfaceType:


[ClassType](ch04-types-values-variables.md#jls-ClassType "ClassType")


TypeVariable:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier")


ArrayType:


[PrimitiveType](ch04-types-values-variables.md#jls-PrimitiveType "PrimitiveType") [Dims](ch04-types-values-variables.md#jls-Dims "Dims")  
[ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType") [Dims](ch04-types-values-variables.md#jls-Dims "Dims")  
[TypeVariable](ch04-types-values-variables.md#jls-TypeVariable "TypeVariable") [Dims](ch04-types-values-variables.md#jls-Dims "Dims")


Dims:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `[` `]` {{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `[` `]`}


The sample code:

``` programlisting

class Point { int[] metrics; }
interface Move { void move(int deltax, int deltay); }
```

declares a class type `Point`, an interface type `Move`, and uses an array type `int``[]` (an array of `int`) to declare the field `metrics` of the class `Point`.


A class or interface type consists of an identifier or a dotted sequence of identifiers, where each identifier is optionally followed by type arguments ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)). If type arguments appear anywhere in a class or interface type, it is a parameterized type ([§4.5](ch04-types-values-variables.md#jls-4.5)).

Each identifier in a class or interface type is classified as a package name or a type name ([§6.5.1](ch06-names.md#jls-6.5.1)). Identifiers which are classified as type names may be annotated. If a class or interface type has the form `T.id` (optionally followed by type arguments), then `id` must be the simple name of an accessible member type of `T` ([§6.6](ch06-names.md#jls-6.6), [§8.5](ch08-classes.md#jls-8.5), [§9.5](ch09-interfaces.md#jls-9.5)), or a compile-time error occurs. The class or interface type denotes that member type.


### 4.3.1. Objects


An *object* is a *class instance* or an *array*.

The reference values (often just *references*) are pointers to these objects, and a special null reference, which refers to no object.

A class instance is explicitly created by a class instance creation expression ([§15.9](ch15-expressions.md#jls-15.9)).

An array is explicitly created by an array creation expression ([§15.10.1](ch15-expressions.md#jls-15.10.1)).

Other expressions may implicitly create a class instance ([§12.5](ch12-execution.md#jls-12.5)) or an array ([§10.6](ch10-arrays.md#jls-10.6)).


**Example 4.3.1-1. Object Creation**


``` programlisting

class Point {
    int x, y;
    Point() { System.out.println("default"); }
    Point(int x, int y) { this.x = x; this.y = y; }

    /* A Point instance is explicitly created at
       class initialization time: */
    static Point origin = new Point(0,0);

    /* A String can be implicitly created
       by a + operator: */
    public String toString() { return "(" + x + "," + y + ")"; }
}

class Test {
    public static void main(String[] args) {
        /* A Point is explicitly created
           using newInstance: */
        Point p = null;
        try {
            p = (Point)Class.forName("Point").newInstance();
        } catch (Exception e) {
            System.out.println(e);
        }

        /* An array is implicitly created
           by an array initializer: */
        Point[] a = { new Point(0,0), new Point(1,1) };

        /* Strings are implicitly created
           by + operators: */
        System.out.println("p: " + p);
        System.out.println("a: { " + a[0] + ", " + a[1] + " }");

        /* An array is explicitly created
           by an array creation expression: */
        String[] sa = new String[2];
        sa[0] = "he"; sa[1] = "llo";
        System.out.println(sa[0] + sa[1]);
    }
}
```

This program produces the output:

``` screen

default
p: (0,0)
a: { (0,0), (1,1) }
hello
```


  

The operators on references to objects are:


- Field access, using either a qualified name ([§6.6](ch06-names.md#jls-6.6)) or a field access expression ([§15.11](ch15-expressions.md#jls-15.11))

- Method invocation ([§15.12](ch15-expressions.md#jls-15.12))

- The cast operator ([§5.5](ch05-conversions-contexts.md#jls-5.5), [§15.16](ch15-expressions.md#jls-15.16))

- The string concatenation operator `+` ([§15.18.1](ch15-expressions.md#jls-15.18.1)), which, when given a `String` operand and a reference, will convert the reference to a `String` by invoking the `toString` method of the referenced object (using `"null"` if either the reference or the result of `toString` is a null reference), and then will produce a newly created `String` that is the concatenation of the two strings

- The `instanceof` operator ([§15.20.2](ch15-expressions.md#jls-15.20.2))

- The reference equality operators `==` and `!=` ([§15.21.3](ch15-expressions.md#jls-15.21.3))

- The conditional operator `? :` ([§15.25](ch15-expressions.md#jls-15.25)).


There may be many references to the same object. Most objects have state, stored in the fields of objects that are instances of classes or in the variables that are the components of an array object. If two variables contain references to the same object, the state of the object can be modified using one variable's reference to the object, and then the altered state can be observed through the reference in the other variable.


**Example 4.3.1-2. Primitive and Reference Identity**


``` programlisting

class Value { int val; }

class Test {
    public static void main(String[] args) {
        int i1 = 3;
        int i2 = i1;
        i2 = 4;
        System.out.print("i1==" + i1);
        System.out.println(" but i2==" + i2);
        Value v1 = new Value();
        v1.val = 5;
        Value v2 = v1;
        v2.val = 6;
        System.out.print("v1.val==" + v1.val);
        System.out.println(" and v2.val==" + v2.val);
    }
}
```

This program produces the output:

``` screen

i1==3 but i2==4
v1.val==6 and v2.val==6
```

because `v1.val` and `v2.val` reference the same instance variable ([§4.12.3](ch04-types-values-variables.md#jls-4.12.3)) in the one `Value` object created by the only `new` expression, while `i1` and `i2` are different variables.


  

Each object is associated with a monitor ([§17.1](ch17-threads-locks.md#jls-17.1)), which is used by `synchronized` methods ([§8.4.3](ch08-classes.md#jls-8.4.3)) and the `synchronized` statement ([§14.19](ch14-blocks-statements-patterns.md#jls-14.19)) to provide control over concurrent access to state by multiple threads ([§17 (Threads and Locks)](ch17-threads-locks.md)).


### 4.3.2. The Class `Object`


The class `Object` is a superclass ([§8.1.4](ch08-classes.md#jls-8.1.4)) of all other classes.

All class and array types inherit ([§8.4.8](ch08-classes.md#jls-8.4.8)) the methods of class `Object`, which are summarized as follows:


- The method `clone` is used to make a duplicate of an object.

- The method `equals` defines a notion of object equality, which is based on value, not reference, comparison.

- The method `finalize` is run just before an object is destroyed ([§12.6](ch12-execution.md#jls-12.6)).

- The method `getClass` returns the `Class` object that represents the class of the object.

  A `Class` object exists for each reference type. It can be used, for example, to discover the fully qualified name of a class, its members, its immediate superclass, and any interfaces that it implements.

  The type of a method invocation expression of `getClass` is `Class``<``?` `extends` \|T\|`>`, where T is the class or interface that was searched for `getClass` ([§15.12.1](ch15-expressions.md#jls-15.12.1)) and \|T\| denotes the erasure of T ([§4.6](ch04-types-values-variables.md#jls-4.6)).

  A class method that is declared `synchronized` ([§8.4.3.6](ch08-classes.md#jls-8.4.3.6)) synchronizes on the monitor associated with the `Class` object of the class.

- The method `hashCode` is very useful, together with the method `equals`, in hashtables such as `java.util.HashMap`.

- The methods `wait`, `notify`, and `notifyAll` are used in concurrent programming using threads ([§17.2](ch17-threads-locks.md#jls-17.2)).

- The method `toString` returns a `String` representation of the object.


### 4.3.3. The Class `String`


Instances of class `String` represent sequences of Unicode code points.

A `String` object has a constant (unchanging) value.

String literals ([§3.10.5](ch03-lexical-structure.md#jls-3.10.5)) and text blocks ([§3.10.6](ch03-lexical-structure.md#jls-3.10.6)) are references to instances of class `String`.

The string concatenation operator `+` ([§15.18.1](ch15-expressions.md#jls-15.18.1)) implicitly creates a new `String` object when the result is not a constant expression ([§15.29](ch15-expressions.md#jls-15.29)).


### 4.3.4. When Reference Types Are the Same


Two reference types are the *same compile-time type* if they are declared in compilation units associated with the same module ([§7.3](ch07-packages-modules.md#jls-7.3)), and they have the same binary name ([§13.1](ch13-binary-compatibility.md#jls-13.1)), and their type arguments, if any, are the same, applying this definition recursively.

When two reference types are the same, they are sometimes said to be the *same class* or the *same interface*.

At run time, several reference types with the same binary name may be loaded simultaneously by different class loaders. These types may or may not represent the same type declaration. Even if two such types do represent the same type declaration, they are considered distinct.

Two reference types are the *same run-time type* if:


- They are both class or both interface types, are defined by the same class loader, and have the same binary name ([§13.1](ch13-binary-compatibility.md#jls-13.1)), in which case they are sometimes said to be the *same run-time class* or the *same run-time interface*.

- They are both array types, and their component types are the same run-time type ([§10 (Arrays)](ch10-arrays.md)).


## 4.4. Type Variables


A *type variable* is an unqualified identifier used as a type in class, interface, method, and constructor bodies.

A type variable is introduced by the declaration of a *type parameter* of a generic class, interface, method, or constructor ([§8.1.2](ch08-classes.md#jls-8.1.2), [§9.1.2](ch09-interfaces.md#jls-9.1.2), [§8.4.4](ch08-classes.md#jls-8.4.4), [§8.8.4](ch08-classes.md#jls-8.8.4)).


TypeParameter:


{[TypeParameterModifier](ch04-types-values-variables.md#jls-TypeParameterModifier "TypeParameterModifier")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeBound](ch04-types-values-variables.md#jls-TypeBound "TypeBound")\]


TypeParameterModifier:


[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")


TypeBound:


`extends` [TypeVariable](ch04-types-values-variables.md#jls-TypeVariable "TypeVariable")  
`extends` [ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType") {[AdditionalBound](ch04-types-values-variables.md#jls-AdditionalBound "AdditionalBound")}


AdditionalBound:


`&` [InterfaceType](ch04-types-values-variables.md#jls-InterfaceType "InterfaceType")


The scope of a type variable declared as a type parameter is specified in [§6.3](ch06-names.md#jls-6.3).

Every type variable declared as a type parameter has a *bound*. If no bound is declared for a type variable, `Object` is assumed. If a bound is declared, it consists of either:


- a single type variable T, or

- a class or interface type T possibly followed by interface types I<sub>1</sub> `&` ... `&` I<sub>n</sub>.


It is a compile-time error if any of the types I<sub>1</sub>, ..., I<sub>n</sub> is a class type or type variable.

The erasures ([§4.6](ch04-types-values-variables.md#jls-4.6)) of all constituent types of a bound must be pairwise different, or a compile-time error occurs.

A type variable must not at the same time be a subtype of two interface types which are different parameterizations of the same generic interface, or a compile-time error occurs.

The order of types in a bound is only significant in that the erasure of a type variable is determined by the first type in its bound, and that a class type or type variable may only appear in the first position.

The members of a type variable X with bound T `&` I<sub>1</sub> `&` ... `&` I<sub>n</sub> are the members of the intersection type ([§4.9](ch04-types-values-variables.md#jls-4.9)) T `&` I<sub>1</sub> `&` ... `&` I<sub>n</sub> appearing at the point where the type variable is declared.


**Example 4.4-1. Members of a Type Variable**


``` programlisting

package TypeVarMembers;

class C {
    public    void mCPublic()    {}
    protected void mCProtected() {}
              void mCPackage()   {}
    private   void mCPrivate()   {}
}

interface I {
    void mI();
}

class CT extends C implements I {
    public void mI() {}
}

class Test {
    <T extends C & I> void test(T t) {
        t.mI();           // OK
        t.mCPublic();     // OK
        t.mCProtected();  // OK
        t.mCPackage();    // OK
        t.mCPrivate();    // Compile-time error
    }
}
```

The type variable `T` has the same members as the intersection type `C & I`, which in turn has the same members as the empty class `CT`, defined in the same scope with equivalent supertypes. The members of an interface are always `public`, and therefore always inherited (unless overridden). Hence `mI` is a member of `CT` and of `T`. Among the members of `C`, all but `mCPrivate` are inherited by `CT`, and are therefore members of both `CT` and `T`.

If `C` had been declared in a different package than `T`, then the call to `mCPackage` would give rise to a compile-time error, as that member would not be accessible at the point where `T` is declared.


  


## 4.5. Parameterized Types


A class or interface that is generic ([§8.1.2](ch08-classes.md#jls-8.1.2), [§9.1.2](ch09-interfaces.md#jls-9.1.2)) defines a set of *parameterized types*.

A parameterized type is a class or interface type of the form C`<`T<sub>1</sub>,...,T<sub>n</sub>`>`, where C is the name of a generic class or interface, and `<`T<sub>1</sub>,...,T<sub>n</sub>`>` is a list of type arguments that denote a particular *parameterization* of the generic class or interface.

A generic class or interface has type parameters F<sub>1</sub>,...,F<sub>n</sub> with corresponding bounds B<sub>1</sub>,...,B<sub>n</sub>. Each type argument T<sub>i</sub> of a parameterized type ranges over all types that are subtypes of all types listed in the corresponding bound. That is, for each bound type S in B<sub>i</sub>, T<sub>i</sub> is a subtype of S`[``F`<sub>`1`</sub>`:=``T`<sub>`1`</sub>`,...,``F`<sub>`n`</sub>`:=``T`<sub>`n`</sub>`]` ([§4.10](ch04-types-values-variables.md#jls-4.10)).

A parameterized type C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` is *well-formed* if all of the following are true:


- C is the name of a generic class or interface.

- The number of type arguments is the same as the number of type parameters in the generic declaration of C.

- When subjected to capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) resulting in the type C`<`X<sub>1</sub>,...,X<sub>n</sub>`>`, each type argument X<sub>i</sub> is a subtype of S`[``F`<sub>`1`</sub>`:=``X`<sub>`1`</sub>`,...,``F`<sub>`n`</sub>`:=``X`<sub>`n`</sub>`]` for each bound type S in B<sub>i</sub>.


It is a compile-time error if a parameterized type is not well-formed.

In this specification, whenever we speak of a class or interface type, we include parameterized types as well, unless explicitly excluded.

Two parameterized types are *provably distinct* if either of the following is true:


- They are parameterizations of distinct generic type declarations.

- Any of their type arguments are provably distinct.


Given the generic classes in the examples of [§8.1.2](ch08-classes.md#jls-8.1.2), here are some well-formed parameterized types:


- `Seq<String>`

- `Seq<Seq<String>>`

- `Seq<String>.Zipper<Integer>`

- `Pair<String,Integer>`


Here are some incorrect parameterizations of those generic classes:


- `Seq<int>` is illegal, as primitive types cannot be type arguments.

- `Pair<String>` is illegal, as there are not enough type arguments.

- `Pair<String,String,String>` is illegal, as there are too many type arguments.


A parameterized type may be a parameterization of a generic class or interface which is nested. For example, if a non-generic class C has a generic member class D with one type parameter, then C`.`D`<``Object``>` is a parameterized type. Meanwhile, if a generic class C with one type parameter has a non-generic member class D, then the member class type C`<``String``>``.`D is a parameterized type, even though the class D is not generic.


### 4.5.1. Type Arguments of Parameterized Types


Type arguments may be either reference types or wildcards. Wildcards are useful in situations where only partial knowledge about the type parameter is required.


TypeArguments:


`<` [TypeArgumentList](ch04-types-values-variables.md#jls-TypeArgumentList "TypeArgumentList") `>`


TypeArgumentList:


[TypeArgument](ch04-types-values-variables.md#jls-TypeArgument "TypeArgument") {`,` [TypeArgument](ch04-types-values-variables.md#jls-TypeArgument "TypeArgument")}


TypeArgument:


[ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType")  
[Wildcard](ch04-types-values-variables.md#jls-Wildcard "Wildcard")


Wildcard:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `?` \[[WildcardBounds](ch04-types-values-variables.md#jls-WildcardBounds "WildcardBounds")\]


WildcardBounds:


`extends` [ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType")  
`super` [ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType")


Wildcards may be given explicit bounds, just like regular type variable declarations. An upper bound is signified by the following syntax, where B is the bound:

``` screen
? extends B
```

Unlike ordinary type variables declared in a method signature, no type inference is required when using a wildcard. Consequently, it is permissible to declare lower bounds on a wildcard, using the following syntax, where B is a lower bound:

``` screen
? super B
```

The wildcard `?` `extends` `Object` is equivalent to the unbounded wildcard `?`.

Two type arguments are *provably distinct* if one of the following is true:


- Neither argument is a type variable or wildcard, and the two arguments are not the same type.

- One type argument is a type variable or wildcard, with a bound (if a type variable) or an upper bound (if a wildcard, using capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)), if necessary) of S; and the other type argument T is not a type variable or wildcard; and neither \|S\| `<:` \|T\| nor \|T\| `<:` \|S\| ([§4.8](ch04-types-values-variables.md#jls-4.8), [§4.10](ch04-types-values-variables.md#jls-4.10)).

- Each type argument is a type variable or wildcard, with upper bounds (from capture conversion, if necessary) of S and T; and neither \|S\| `<:` \|T\| nor \|T\| `<:` \|S\|.


A type argument T<sub>1</sub> is said to *contain* another type argument T<sub>2</sub>, written T<sub>2</sub> `<=` T<sub>1</sub>, if the set of types denoted by T<sub>2</sub> is provably a subset of the set of types denoted by T<sub>1</sub> under the reflexive and transitive closure of the following rules (where `<:` denotes subtyping ([§4.10](ch04-types-values-variables.md#jls-4.10))):


- `?` `extends` T `<=` `?` `extends` S if T `<:` S

- `?` `extends` T `<=` `?`

- `?` `super` T `<=` `?` `super` S if S `<:` T

- `?` `super` T `<=` `?`

- `?` `super` T `<=` `?` `extends` `Object`

- T `<=` T

- T `<=` `?` `extends` T

- T `<=` `?` `super` T


The relationship of wildcards to established type theory is an interesting one, which we briefly allude to here. Wildcards are a restricted form of existential types. Given a generic type declaration G`<`T `extends` B`>`, G`<``?``>` is roughly analogous to Some X `<:` B. G`<`X`>`.

Historically, wildcards are a direct descendant of the work by Atsushi Igarashi and Mirko Viroli. Readers interested in a more comprehensive discussion should refer to *On Variance-Based Subtyping for Parametric Types* by Atsushi Igarashi and Mirko Viroli, in the *Proceedings of the 16th European Conference on Object Oriented Programming (ECOOP 2002)*. This work itself builds upon earlier work by Kresten Thorup and Mads Torgersen (*Unifying Genericity*, ECOOP 99), as well as a long tradition of work on declaration based variance that goes back to Pierre America's work on POOL (OOPSLA 89).

Wildcards differ in certain details from the constructs described in the aforementioned paper, in particular in the use of capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) rather than the `close` operation described by Igarashi and Viroli. For a formal account of wildcards, see *Wild FJ* by Mads Torgersen, Erik Ernst and Christian Plesner Hansen, in the 12th workshop on Foundations of Object Oriented Programming (FOOL 2005).


**Example 4.5.1-1. Unbounded Wildcards**


``` programlisting

import java.util.ArrayList;
import java.util.Collection;

class Test {
    static void printCollection(Collection<?> c) {
                                // a wildcard collection
        for (Object o : c) {
            System.out.println(o);
        }
    }

    public static void main(String[] args) {
        Collection<String> cs = new ArrayList<String>();
        cs.add("hello");
        cs.add("world");
        printCollection(cs);
    }
}
```

Note that using `Collection<Object>` as the type of the incoming parameter, `c`, would not be nearly as useful; the method could only be used with an argument expression that had type `Collection<Object>`, which would be quite rare. In contrast, the use of an unbounded wildcard allows any kind of collection to be passed as an argument.

Here is an example where the element type of an array is parameterized by a wildcard:

``` programlisting


public Method getMethod(Class<?>[] parameterTypes) { ... }
```


  


**Example 4.5.1-2. Bounded Wildcards**


``` screen
boolean addAll(Collection<? extends E> c)
```

Here, the method is declared within the interface `Collection<E>`, and is designed to add all the elements of its incoming argument to the collection upon which it is invoked. A natural tendency would be to use `Collection<E>` as the type of `c`, but this is unnecessarily restrictive. An alternative would be to declare the method itself to be generic:

``` screen
<T> boolean addAll(Collection<T> c)
```

This version is sufficiently flexible, but note that the type parameter is used only once in the signature. This reflects the fact that the type parameter is not being used to express any kind of interdependency between the type(s) of the argument(s), the return type and/or throws type. In the absence of such interdependency, generic methods are considered bad style, and wildcards are preferred.

``` screen
Reference(T referent, ReferenceQueue<? super T> queue)
```

Here, the referent can be inserted into any queue whose element type is a supertype of the type `T` of the referent; `T` is the lower bound for the wildcard.


  


### 4.5.2. Members and Constructors of Parameterized Types


Let C be a generic class or interface with type parameters A<sub>1</sub>,...,A<sub>n</sub>, and let C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` be a parameterization of C where, for 1 ≤ *i* ≤ *n*, T<sub>i</sub> is a type (rather than a wildcard). Then:


- Let `m` be a member or constructor declaration in C, whose type as declared is T ([§8.2](ch08-classes.md#jls-8.2), [§8.8.6](ch08-classes.md#jls-8.8.6)).

  The type of `m` in C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` is T`[``A`<sub>`1`</sub>`:=``T`<sub>`1`</sub>`,...,``A`<sub>`n`</sub>`:=``T`<sub>`n`</sub>`]`.

- Let `m` be a member or constructor declaration in D, where D is a class extended by C or an interface implemented by C. Let D`<`U<sub>1</sub>,...,U<sub>k</sub>`>` be the supertype ([§4.10.2](ch04-types-values-variables.md#jls-4.10.2)) of C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` that corresponds to D.

  The type of `m` in C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` is the type of `m` in D`<`U<sub>1</sub>,...,U<sub>k</sub>`>`.


If any of the type arguments in the parameterization of C are wildcards, then:


- The types of the fields, methods, and constructors in C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` are the types of the fields, methods, and constructors in the capture conversion of C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)).

- Let D be a (possibly generic) class or interface declaration in C. Then the type of D in C`<`T<sub>1</sub>,...,T<sub>n</sub>`>` is D where, if D is generic, all type arguments are unbounded wildcards.


This is of no consequence, as it is impossible to access a member of a parameterized type without performing capture conversion, and it is impossible to use a wildcard after the keyword `new` in a class instance creation expression ([§15.9](ch15-expressions.md#jls-15.9)).

The sole exception to the previous paragraph is when a nested parameterized type is used as the expression in an `instanceof` operator ([§15.20.2](ch15-expressions.md#jls-15.20.2)), where capture conversion is not applied.

A `static` member that is declared in a generic class or interface must be referred to using the name of the generic class or interface ([§6.1](ch06-names.md#jls-6.1), [§6.5.5.2](ch06-names.md#jls-6.5.5.2), [§6.5.6.2](ch06-names.md#jls-6.5.6.2)), or a compile-time error occurs.

In other words, it is illegal to refer to a `static` member declared in a generic type declaration by using a parameterized type.


## 4.6. Type Erasure


Type erasure is a mapping from types (possibly including parameterized types and type variables) to types (that are never parameterized types or type variables). We write \|T\| for the erasure of type T. The erasure mapping is defined as follows:


- The erasure of a parameterized type ([§4.5](ch04-types-values-variables.md#jls-4.5)) G`<`T<sub>1</sub>,...,T<sub>n</sub>`>` is \|G\|.

- The erasure of a nested type T`.`C is \|T\|.C.

- The erasure of an array type T`[]` is \|T\|`[]`.

- The erasure of a type variable ([§4.4](ch04-types-values-variables.md#jls-4.4)) is the erasure of its leftmost bound.

- The erasure of every other type is the type itself.


Type erasure also maps the signature ([§8.4.2](ch08-classes.md#jls-8.4.2)) of a constructor or method to a signature that has no parameterized types or type variables. The erasure of a constructor or method signature s is a signature consisting of the same name as s and the erasures of all the formal parameter types given in s.

The return type of a method ([§8.4.5](ch08-classes.md#jls-8.4.5)) and the type parameters of a generic method or constructor ([§8.4.4](ch08-classes.md#jls-8.4.4), [§8.8.4](ch08-classes.md#jls-8.8.4)) also undergo erasure if the method or constructor's signature is erased.

The erasure of the signature of a generic method has no type parameters.


## 4.7. Reifiable Types


Because some type information is erased during compilation, not all types are available at run time. Types that are completely available at run time are known as *reifiable types*.

A type is *reifiable* if and only if one of the following holds:


- It refers to a non-generic class or interface type declaration.

- It is a parameterized type in which all type arguments are unbounded wildcards ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)).

- It is a raw type ([§4.8](ch04-types-values-variables.md#jls-4.8)).

- It is a primitive type ([§4.2](ch04-types-values-variables.md#jls-4.2)).

- It is an array type ([§10.1](ch10-arrays.md#jls-10.1)) whose element type is reifiable.

- It is a nested type where, for each type T separated by a "`.`", T itself is reifiable.

  For example, if a generic class X`<`T`>` has a generic member class Y`<`U`>`, then the type X`<``?``>``.`Y`<``?``>` is reifiable because X`<``?``>` is reifiable and Y`<``?``>` is reifiable. The type X`<``?``>``.`Y`<``Object``>` is not reifiable because Y`<``Object``>` is not reifiable.


An intersection type is not reifiable.

The decision not to make all generic types reifiable is one of the most crucial, and controversial design decisions involving the type system of the Java programming language.

Ultimately, the most important motivation for this decision is compatibility with existing code. In a naive sense, the addition of new constructs such as generics has no implications for pre-existing code. The Java programming language, per se, is compatible with earlier versions as long as every program written in the previous versions retains its meaning in the new version. However, this notion, which may be termed language compatibility, is of purely theoretical interest. Real programs (even trivial ones, such as "Hello World") are composed of several compilation units, some of which are provided by the Java SE Platform (such as elements of `java.lang` or `java.util`). In practice, then, the minimum requirement is platform compatibility - that any program written for the prior version of the Java SE Platform continues to function unchanged in the new version.

One way to provide platform compatibility is to leave existing platform functionality unchanged, only adding new functionality. For example, rather than modify the existing Collections hierarchy in `java.util`, one might introduce a new library utilizing generics.

The disadvantages of such a scheme is that it is extremely difficult for pre-existing clients of the Collection library to migrate to the new library. Collections are used to exchange data between independently developed modules; if a vendor decides to switch to the new, generic, library, that vendor must also distribute two versions of their code, to be compatible with their clients. Libraries that are dependent on other vendors code cannot be modified to use generics until the supplier's library is updated. If two modules are mutually dependent, the changes must be made simultaneously.

Clearly, platform compatibility, as outlined above, does not provide a realistic path for adoption of a pervasive new feature such as generics. Therefore, the design of the generic type system seeks to support migration compatibility. Migration compatibility allows the evolution of existing code to take advantage of generics without imposing dependencies between independently developed software modules.

The price of migration compatibility is that a full and sound reification of the generic type system is not possible, at least while the migration is taking place.


## 4.8. Raw Types


To facilitate interfacing with non-generic legacy code, it is possible to use as a type the erasure ([§4.6](ch04-types-values-variables.md#jls-4.6)) of a parameterized type ([§4.5](ch04-types-values-variables.md#jls-4.5)) or the erasure of an array type ([§10.1](ch10-arrays.md#jls-10.1)) whose element type is a parameterized type. Such a type is called a *raw type*.

More precisely, a raw type is defined to be one of:


- The reference type that is formed by taking the name of a generic class or interface declaration without an accompanying type argument list.

- An array type whose element type is a raw type.

- The name of an inner member class of a raw type R that is not inherited from a superclass or superinterface of R.


The type of a non-generic class or interface is not a raw type.


To see why the name of an inner member class of a raw type is considered raw, consider the following example:

``` programlisting

class Outer<T>{
    T t;
    class Inner {
        T setOuterT(T t1) { t = t1; return t; }
    }
}
```

The type of the member(s) of `Inner` depends on the type parameter of `Outer`. If `Outer` is raw, `Inner` must be treated as raw as well, as there is no valid binding for `T`.

This rule applies only to inner member classes that are not inherited. Inherited inner member classes that depend on type variables will be inherited as raw types as a consequence of the rule that the supertypes of a raw type are erased, described later in this section.

Another implication of the rules above is that a generic inner class of a raw type can itself only be used as a raw type:

``` programlisting

class Outer<T>{
    class Inner<S> {
        S s;
    }
}
```

It is not possible to access `Inner` as a partially raw type (a "rare" type):

``` screen

Outer.Inner<Double> x = null;  // illegal
Double d = x.s;
```

because `Outer` itself is raw, hence so are all its inner classes including `Inner`, and so it is not possible to pass any type arguments to Inner.


The superclass types (respectively, superinterface types) of a raw type are the erasures of the superclass types (superinterface types) of the named class or interface.

The type of a constructor ([§8.8](ch08-classes.md#jls-8.8)), instance method ([§8.4](ch08-classes.md#jls-8.4), [§9.4](ch09-interfaces.md#jls-9.4)), or non-`static` field ([§8.3](ch08-classes.md#jls-8.3)) of a raw type C that is not inherited from its superclasses or superinterfaces is the erasure of its type in the generic class or interface C.

The type of an inherited instance method or non-`static` field of a raw type C, where the member was declared in a class or interface D, is the type of the member in the supertype of C that names D.

The type of a `static` method or `static` field of a raw type C is the same as its type in the generic class or interface C.

It is a compile-time error to pass type arguments to a non-`static` member class or interface of a raw type that is not inherited from its superclasses or superinterfaces.

It is a compile-time error to attempt to use a member class or interface of a parameterized type as a raw type.


This means that the ban on "rare" types extends to the case where the qualifying type is parameterized, but we attempt to use the inner class as a raw type:

``` screen

Outer<Integer>.Inner x = null; // illegal
```

This is the opposite of the case discussed above. There is no practical justification for this half-baked type. In legacy code, no type arguments are used. In non-legacy code, we should use the generic types correctly and pass all the required type arguments.


The use of raw types is allowed only as a concession to compatibility of legacy code. The use of raw types in code written after the introduction of generics into the Java programming language is strongly discouraged. It is possible that future versions of the Java programming language will disallow the use of raw types.

To make sure that potential violations of the typing rules are always flagged, some accesses to members of a raw type will result in compile-time unchecked warnings. The rules for compile-time unchecked warnings when accessing members or constructors of raw types are as follows:


- At an assignment to a field: if the type of the *Primary* in the field access expression ([§15.11](ch15-expressions.md#jls-15.11)) is a raw type, then a compile-time unchecked warning occurs if erasure changes the field's type.

- At an invocation of a method or constructor: if the type of the class or interface to search ([§15.12.1](ch15-expressions.md#jls-15.12.1)) is a raw type, then a compile-time unchecked warning occurs if erasure changes any of the formal parameter types of the method or constructor.

- No compile-time unchecked warning occurs for a method call when the formal parameter types do not change under erasure (even if the return type and/or `throws` clause changes), for reading from a field, or for a class instance creation of a raw type.


Note that the unchecked warnings above are distinct from the unchecked warnings possible from narrowing reference conversion ([§5.1.6](ch05-conversions-contexts.md#jls-5.1.6)), unchecked conversion ([§5.1.9](ch05-conversions-contexts.md#jls-5.1.9)), method declarations ([§8.4.1](ch08-classes.md#jls-8.4.1), [§8.4.8.3](ch08-classes.md#jls-8.4.8.3)), and certain expressions ([§15.12.4.2](ch15-expressions.md#jls-15.12.4.2), [§15.13.2](ch15-expressions.md#jls-15.13.2), [§15.27.3](ch15-expressions.md#jls-15.27.3)).

The warnings here cover the case where a legacy consumer uses a generified library. For example, the library declares a generic class `Foo<T extends String>` that has a field `f` of type `Vector<T>`, but the consumer assigns a vector of integers to `e``.``f` where `e` has the raw type `Foo`. The legacy consumer receives a warning because it may have caused heap pollution ([§4.12.2](ch04-types-values-variables.md#jls-4.12.2)) for generified consumers of the generified library.

(Note that the legacy consumer can assign a `Vector<String>` from the library to its own `Vector` variable without receiving a warning. That is, the subtyping rules ([§4.10.2](ch04-types-values-variables.md#jls-4.10.2)) of the Java programming language make it possible for a variable of a raw type to be assigned a value of any of the type's parameterized instances.)

The warnings from unchecked conversion cover the dual case, where a generified consumer uses a legacy library. For example, a method of the library has the raw return type `Vector`, but the consumer assigns the result of the method invocation to a variable of type `Vector<String>`. This is unsafe, since the raw vector might have had a different element type than `String`, but is still permitted using unchecked conversion in order to enable interfacing with legacy code. The warning from unchecked conversion indicates that the generified consumer may experience problems from heap pollution at other points in the program.


**Example 4.8-1. Raw Types**


``` programlisting

class Cell<E> {
    E value;

    Cell(E v)     { value = v; }
    E get()       { return value; }
    void set(E v) { value = v; }

    public static void main(String[] args) {
        Cell x = new Cell<String>("abc");
        System.out.println(x.value);  // OK, has type Object
        System.out.println(x.get());  // OK, has type Object
        x.set("def");                 // unchecked warning
    }
}
```


  


**Example 4.8-2. Raw Types and Inheritance**


``` programlisting

import java.util.ArrayList;
import java.util.Collection;
import java.util.Iterator;

class NonGeneric {
    Collection<Number> myNumbers() { return null; }
}

abstract class RawMembers<T> extends NonGeneric
                             implements Collection<String> {
    static Collection<NonGeneric> cng =
        new ArrayList<NonGeneric>();

    public static void main(String[] args) {
        RawMembers rw = null;

        Collection<Number> cn = rw.myNumbers();
                                 // OK

        Iterator<String> is   = rw.iterator();
                                 // Unchecked warning

        Collection<NonGeneric> cnn = rw.cng;
                                      // OK, static member
    }
}
```

In this program (which is not meant to be run), `RawMembers<T>` inherits the method:

``` screen

Iterator<String> iterator()
```

from the `Collection<String>` superinterface. The raw type `RawMembers` inherits `iterator()` from `Collection`, the erasure of `Collection<String>`, which means that the return type of `iterator()` in `RawMembers` is `Iterator`. As a result, the attempt to assign `rw.iterator()` to `Iterator<String>` requires an unchecked conversion, so a compile-time unchecked warning is issued.

In contrast, `RawMembers` inherits `myNumbers()` from the `NonGeneric` class whose erasure is also `NonGeneric`. Thus, the return type of `myNumbers()` in `RawMembers` is not erased, and the attempt to assign `rw.myNumbers()` to `Collection<Number>` requires no unchecked conversion, so no compile-time unchecked warning is issued.

Similarly, the `static` member `cng` retains its parameterized type even when accessed through a object of raw type. Note that access to a `static` member through an instance is considered bad style and is discouraged.

This example reveals that certain members of a raw type are not erased, namely `static` members whose types are parameterized, and members inherited from a non-generic supertype.


  

Raw types are closely related to wildcards. Both are based on existential types. Raw types can be thought of as wildcards whose type rules are deliberately unsound, to accommodate interaction with legacy code. Historically, raw types preceded wildcards; they were first introduced in GJ, and described in the paper *Making the future safe for the past: Adding Genericity to the Java Programming Language* by Gilad Bracha, Martin Odersky, David Stoutamire, and Philip Wadler, in *Proceedings of the ACM Conference on Object-Oriented Programming, Systems, Languages and Applications (OOPSLA 98)*, October 1998.


## 4.9. Intersection Types


An intersection type takes the form T<sub>1</sub> `&` ... `&` T<sub>n</sub> (*n* \> 0), where T<sub>i</sub> (1 ≤ *i* ≤ *n*) are types.

Intersection types can be derived from type parameter bounds ([§4.4](ch04-types-values-variables.md#jls-4.4)) and cast expressions ([§15.16](ch15-expressions.md#jls-15.16)); they also arise in the processes of capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) and least upper bound computation ([§4.10.4](ch04-types-values-variables.md#jls-4.10.4)).

The values of an intersection type are those objects that are values of all of the types T<sub>i</sub> for 1 ≤ *i* ≤ *n*.

Every intersection type T<sub>1</sub> `&` ... `&` T<sub>n</sub> *induces* a notional class or interface for the purpose of identifying the members of the intersection type, as follows:


- For each T<sub>i</sub> (1 ≤ *i* ≤ *n*), let C<sub>i</sub> be the most specific class or array type such that T<sub>i</sub> `<:` C<sub>i</sub>. Then there must be some C<sub>k</sub> such that C<sub>k</sub> `<:` C<sub>i</sub> for any *i* (1 ≤ *i* ≤ *n*), or a compile-time error occurs.

- For 1 ≤ *j* ≤ *n*, if T<sub>j</sub> is a type variable, then let T<sub>j</sub>' be an interface whose members are the same as the `public` members of T<sub>j</sub>; otherwise, if T<sub>j</sub> is an interface, then let T<sub>j</sub>' be T<sub>j</sub>.

- If C<sub>k</sub> is `Object`, a notional interface is induced; otherwise, a notional class is induced with direct superclass type C<sub>k</sub>. This class or interface has direct superinterface types T<sub>1</sub>', ..., T<sub>n</sub>' and is declared in the package in which the intersection type appears.


The members of an intersection type are the members of the class or interface it induces.

It is worth dwelling upon the distinction between intersection types and the bounds of type variables. Every type variable bound induces an intersection type. This intersection type is often trivial, consisting of a single type. The form of a bound is restricted (only the first element may be a class or type variable, and only one type variable may appear in the bound) to preclude certain awkward situations coming into existence. However, capture conversion can lead to the creation of type variables whose bounds are more general, such as array types).


## 4.10. Subtyping


The subtype and supertype relations are binary relations on types.

The *supertypes* of a type are obtained by reflexive and transitive closure over the direct supertype relation, written S `>`<sub>`1`</sub> T, which is defined by rules given later in this section. We write S `:>` T to indicate that the supertype relation holds between S and T.

S is a *proper supertype* of T, written S `>` T, if S `:>` T and S ≠ T.

The *subtypes* of a type T are all types U such that T is a supertype of U, and the null type. We write T `<:` S to indicate that that the subtype relation holds between types T and S.

T is a *proper subtype* of S, written T `<` S, if T `<:` S and S ≠ T.

T is a *direct subtype* of S, written T `<`<sub>`1`</sub> S, if S `>`<sub>`1`</sub> T.

Subtyping does not extend through parameterized types: T `<:` S does not imply that C`<`T`>` `<:` C`<`S`>`.


### 4.10.1. Subtyping among Primitive Types


The following rules define the direct supertype relation among the primitive types:


- `double` `>`<sub>`1`</sub> `float`

- `float` `>`<sub>`1`</sub> `long`

- `long` `>`<sub>`1`</sub> `int`

- `int` `>`<sub>`1`</sub> `char`

- `int` `>`<sub>`1`</sub> `short`

- `short` `>`<sub>`1`</sub> `byte`


### 4.10.2. Subtyping among Class and Interface Types


Given a non-generic class or interface C, the *direct supertypes* of the type of C are all of the following:


- The direct superclass type of C ([§8.1.4](ch08-classes.md#jls-8.1.4)), if C is a class.

- The direct superinterface types of C ([§8.1.5](ch08-classes.md#jls-8.1.5), [§9.1.3](ch09-interfaces.md#jls-9.1.3)).

- The type `Object`, if C is an interface with no direct superinterface types ([§9.1.3](ch09-interfaces.md#jls-9.1.3)).


Given a generic class or interface C with type parameters F<sub>1</sub>,...,F<sub>n</sub> (*n* \> 0), the *direct supertypes* of the raw type C ([§4.8](ch04-types-values-variables.md#jls-4.8)) are all of the following:


- The erasure ([§4.6](ch04-types-values-variables.md#jls-4.6)) of the direct superclass type of C, if C is a class.

- The erasure of the direct superinterface types of C.

- The type `Object`, if C is an interface with no direct superinterface types.


Given a generic class or interface C with type parameters F<sub>1</sub>,...,F<sub>n</sub> (*n* \> 0), the *direct supertypes* of the parameterized type C`<`T<sub>1</sub>,...,T<sub>n</sub>`>`, where each of T<sub>i</sub> (1 ≤ *i* ≤ *n*) is a type, are all of the following:


- The substitution `[``F`<sub>`1`</sub>`:=``T`<sub>`1`</sub>`,...,``F`<sub>`n`</sub>`:=``T`<sub>`n`</sub>`]` applied to the direct superclass type of C, if C is a class.

- The substitution `[``F`<sub>`1`</sub>`:=``T`<sub>`1`</sub>`,...,``F`<sub>`n`</sub>`:=``T`<sub>`n`</sub>`]` applied to the direct superinterface types of C.

- C`<`S<sub>1</sub>,...,S<sub>n</sub>`>`, where S<sub>i</sub> contains T<sub>i</sub> (1 ≤ *i* ≤ *n*) ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)).

- The type `Object`, if C is an interface with no direct superinterface types.

- The raw type C.


Given a generic class or interface C with type parameters F<sub>1</sub>,...,F<sub>n</sub> (*n* \> 0), the *direct supertypes* of the parameterized type C`<`R<sub>1</sub>,...,R<sub>n</sub>`>` where at least one of the R<sub>i</sub> (1 ≤ *i* ≤ *n*) is a wildcard type argument, are the direct supertypes of the parameterized type C`<`X<sub>1</sub>,...,X<sub>n</sub>`>` which is the result of applying capture conversion to C`<`R<sub>1</sub>,...,R<sub>n</sub>`>` ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)).

The direct supertypes of an intersection type T<sub>1</sub> `&` ... `&` T<sub>n</sub> are T<sub>i</sub> (1 ≤ *i* ≤ *n*).

The direct supertypes of a type variable are the types listed in its bound.

A type variable is a direct supertype of its lower bound.

The direct supertypes of the null type are all reference types other than the null type itself.


### 4.10.3. Subtyping among Array Types


The following rules define the direct supertype relation among array types:


- If S and T are both reference types, then S`[]` `>`<sub>`1`</sub> T`[]` iff S `>`<sub>`1`</sub> T.

- `Object` `>`<sub>`1`</sub> `Object``[]`

- `Cloneable` `>`<sub>`1`</sub> `Object``[]`

- `java.io.Serializable` `>`<sub>`1`</sub> `Object``[]`

- If P is a primitive type, then:


  - `Object` `>`<sub>`1`</sub> P`[]`

  - `Cloneable` `>`<sub>`1`</sub> P`[]`

  - `java.io.Serializable` `>`<sub>`1`</sub> P`[]`

  


### 4.10.4. Least Upper Bound


The *least upper bound*, or "lub", of a set of reference types is a shared supertype that is more specific than any other shared supertype (that is, no other shared supertype is a subtype of the least upper bound). This type, lub(U<sub>1</sub>, ..., U<sub>k</sub>), is determined as follows.

If *k* = 1, then the lub is the type itself: lub(U) = U.

Otherwise:


- For each U<sub>i</sub> (1 ≤ *i* ≤ *k*):

  Let ST(U<sub>i</sub>) be the set of supertypes of U<sub>i</sub>.

  Let EST(U<sub>i</sub>), the set of erased supertypes of U<sub>i</sub>, be:

  EST(U<sub>i</sub>) = { \|W\| \| W in ST(U<sub>i</sub>) } where \|W\| is the erasure of W.

  The reason for computing the set of erased supertypes is to deal with situations where the set of types includes several distinct parameterizations of a generic type.

  For example, given `List<String>` and `List<Object>`, simply intersecting the sets ST(`List<String>`) = { `List<String>`, `Collection<String>`, `Object` } and ST(`List<Object>`) = { `List<Object>`, `Collection<Object>`, `Object` } would yield a set { `Object` }, and we would have lost track of the fact that the upper bound can safely be assumed to be a `List`.

  In contrast, intersecting EST(`List<String>`) = { `List`, `Collection`, `Object` } and EST(`List<Object>`) = { `List`, `Collection`, `Object` } yields { `List`, `Collection`, `Object` }, which will eventually enable us to produce `List<?>`.

- Let EC, the erased candidate set for U<sub>1</sub>, ..., U<sub>k</sub>, be the intersection of all the sets EST(U<sub>i</sub>) (1 ≤ *i* ≤ *k*).

- Let MEC, the minimal erased candidate set for U<sub>1</sub>, ..., U<sub>k</sub>, be:

  MEC = { V \| V in EC, and for all W ≠ V in EC, it is not the case that W `<:` V }

  Because we are seeking to infer more precise types, we wish to filter out any candidates that are supertypes of other candidates. This is what computing MEC accomplishes. In our running example, we had EC = { `List`, `Collection`, `Object` }, so MEC = { `List` }. The next step is to recover type arguments for the erased types in MEC.

- For any element G of MEC that is a generic type:

  Let the "relevant" parameterizations of G, Relevant(G), be:

  Relevant(G) = { V \| 1 ≤ *i* ≤ *k*: V in ST(U<sub>i</sub>) and V = G`<`...`>` }

  In our running example, the only generic element of MEC is `List`, and Relevant(`List`) = { `List<String>`, `List<Object>` }. We will now seek to find a type argument for `List` that contains ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)) both `String` and `Object`.

  This is done by means of the least containing parameterization (lcp) operation defined below. The first line defines lcp() on a set, such as Relevant(`List`), as an operation on a list consisting of the elements of the set. The next line defines the operation on such a list as a pairwise reduction on the elements of the list. The third line is the definition of lcp() on pairs of parameterized types, which in turn relies on the notion of least containing type argument (lcta). lcta() is defined for all possible cases.

  Let the "candidate" parameterization of G, Candidate(G), be the most specific parameterization of the generic type G that contains all the relevant parameterizations of G:

  Candidate(G) = lcp(Relevant(G))

  where lcp(), the least containing parameterization, is:


  - lcp(S) = lcp(`e`<sub>`1`</sub>, ..., `e`<sub>`n`</sub>) where `e`<sub>`i`</sub> (1 ≤ *i* ≤ *n*) in S

  - lcp(`e`<sub>`1`</sub>, ..., `e`<sub>`n`</sub>) = lcp(lcp(`e`<sub>`1`</sub>, `e`<sub>`2`</sub>), `e`<sub>`3`</sub>, ..., `e`<sub>`n`</sub>)

  - lcp(G`<`X<sub>1</sub>, ..., X<sub>n</sub>`>`, G`<`Y<sub>1</sub>, ..., Y<sub>n</sub>`>`) = G`<`lcta(X<sub>1</sub>, Y<sub>1</sub>), ..., lcta(X<sub>n</sub>, Y<sub>n</sub>)`>`

  - lcp(G`<`X<sub>1</sub>, ..., X<sub>n</sub>`>`) = G`<`lcta(X<sub>1</sub>), ..., lcta(X<sub>n</sub>)`>`

  

  and where lcta(), the least containing type argument, is: (assuming U and V are types)


  - lcta(U, V) = U if U = V, otherwise `?` `extends` lub(U, V)

  - lcta(U, `?` `extends` V) = `?` `extends` lub(U, V)

  - lcta(U, `?` `super` V) = `?` `super` glb(U, V)

  - lcta(`?` `extends` U, `?` `extends` V) = `?` `extends` lub(U, V)

  - lcta(`?` `extends` U, `?` `super` V) = `?`

  - lcta(`?` `super` U, `?` `super` V) = `?` `super` glb(U, V)

  - lcta(U) = `?` if U's upper bound is `Object`, otherwise `?` `extends` lub(U,`Object`)

  

  and where glb() is as defined in [§5.1.10](ch05-conversions-contexts.md#jls-5.1.10).

- Let lub(U<sub>1</sub>, ..., U<sub>k</sub>) be:

  Best(W<sub>1</sub>) `&` ... `&` Best(W<sub>r</sub>)

  where W<sub>i</sub> (1 ≤ *i* ≤ *r*) are the elements of MEC, the minimal erased candidate set of U<sub>1</sub>, ..., U<sub>k</sub>;

  and where, if any of these elements are generic, we use the candidate parameterization (so as to recover type arguments):

  Best(X) = Candidate(X) if X is generic; X otherwise.


Strictly speaking, this lub() function only approximates a least upper bound. Formally, there may exist some other type T such that all of U<sub>1</sub>, ..., U<sub>k</sub> are subtypes of T and T is a subtype of lub(U<sub>1</sub>, ..., U<sub>k</sub>). However, a compiler for the Java programming language must implement lub() as specified above.

It is possible that the lub() function yields an infinite type. This is permissible, and a compiler for the Java programming language must recognize such situations and represent them appropriately using cyclic data structures.

The possibility of an infinite type stems from the recursive calls to lub(). Readers familiar with recursive types should note that an infinite type is not the same as a recursive type.


### 4.10.5. Type Projections


A *synthetic type variable* is a type variable introduced by the compiler during capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) or inference variable resolution ([§18.4](ch18-type-inference.md#jls-18.4)).

It is sometimes necessary to find a close supertype of a type, where that supertype does not mention certain synthetic type variables. This is achieved with an *upward projection* applied to the type.

Similarly, a *downward projection* may be applied to find a close subtype of a type, where that subtype does not mention certain synthetic type variables. Because such a type does not always exist, downward projection is a partial function.

These operations take as input a set of type variables that should no longer be referenced, referred to as the *restricted type variables*. When the operations recur, the set of restricted type variables is implicitly passed on to the recursive application.

The upward projection of a type T with respect to a set of restricted type variables is defined as follows:


- If T does not mention any restricted type variable, then the result is T.

- If T is a restricted type variable, then the result is the upward projection of the upper bound of T.

- If T is a parameterized class type or a parameterized interface type, G`<`A<sub>1</sub>,...,A<sub>n</sub>`>`, then the result is G`<`A<sub>1</sub>',...,A<sub>n</sub>'`>`, where, for 1 ≤ *i* ≤ *n*, A<sub>i</sub>' is derived from A<sub>i</sub> as follows:


  - If A<sub>i</sub> does not mention any restricted type variable, then A<sub>i</sub>' = A<sub>i</sub>.

  - If A<sub>i</sub> is a type that mentions a restricted type variable, then let U be the upward projection of A<sub>i</sub>. A<sub>i</sub>' is a wildcard, defined by three cases:


    - If U is not `Object`, and if either the declared bound of the *i*th parameter of G, B<sub>i</sub>, mentions a type parameter of G, or B<sub>i</sub> is not a subtype of U, then A<sub>i</sub>' is an upper-bounded wildcard, `?` `extends` U.

    - Otherwise, if the downward projection of A<sub>i</sub> is `L`, then A<sub>i</sub>' is a lower-bounded wildcard, `?` `super` `L`.

    - Otherwise, the downward projection of A<sub>i</sub> is undefined and A<sub>i</sub>' is an unbounded wildcard, `?`.

    

  - If A<sub>i</sub> is an upper-bounded wildcard that mentions a restricted type variable, then let U be the upward projection of the wildcard bound. A<sub>i</sub>' is an upper-bounded wildcard, `?` `extends` U.

  - If A<sub>i</sub> is a lower-bounded wildcard that mentions a restricted type variable, then if the downward projection of the wildcard bound is `L`, then A<sub>i</sub>' is a lower-bounded wildcard, `?` `super` `L`; if the downward projection of the wildcard bound is undefined, then A<sub>i</sub>' is an unbounded wildcard, `?`.

  

- If T is an array type, S`[]`, then the result is an array type whose component type is the upward projection of S.

- If T is an intersection type, then the result is an intersection type. For each element, S, of T, the result has as an element the upward projection of S.


The downward projection of a type T with respect to a set of restricted type variables is a partial function, defined as follows:


- If T does not mention any restricted type variable, then the result is T.

- If T is a restricted type variable, then if T has a lower bound, and if the downward projection of that bound is `L`, the result is `L`; if T has no lower bound, or if the downward projection of that bound is undefined, then the result is undefined.

- If T is a parameterized class type or a parameterized interface type, G`<`A<sub>1</sub>,...,A<sub>n</sub>`>`, then the result is G`<`A<sub>1</sub>',...,A<sub>n</sub>'`>`, if, for 1 ≤ *i* ≤ *n*, a type argument A<sub>i</sub>' can be derived from A<sub>i</sub> as follows; if not, the result is undefined:


  - If A<sub>i</sub> is does not mention a restricted type variable, then A<sub>i</sub>' = A<sub>i</sub>.

  - If A<sub>i</sub> is a type that mentions a restricted type variable, then A<sub>i</sub>' is undefined.

  - If A<sub>i</sub> is an upper-bounded wildcard that mentions a restricted type variable, then if the downward projection of the wildcard bound is U, then A<sub>i</sub>' is an upper-bounded wildcard, `?` `extends` U; if the downward projection of the wildcard bound is undefined, then A<sub>i</sub>' is undefined.

  - If A<sub>i</sub> is a lower-bounded wildcard that mentions a restricted type variable, then let `L` be the upward projection of the wildcard bound. A<sub>i</sub>' is a lower-bounded wildcard, `?` `super` `L`.

  

- If T is an array type, S`[]`, then if the downward projection of S is S', the result is S'`[]`; if the downward projection of S is undefined, then the result is undefined.

- If T is an intersection type, then if the downward projection is defined for *each* element of T, the result is an intersection type whose elements are the downward projections of the elements of T; if the downward projection is undefined for *any* element of T, then the result is undefined.


Like lub ([§4.10.4](ch04-types-values-variables.md#jls-4.10.4)), upward projection and downward projection may produce infinite types, due to the recursion on type variable bounds.


## 4.11. Where Types Are Used


Types are used in most kinds of declaration and in certain kinds of expression. Specifically, there are 17 *type contexts* where types are used:


- In declarations:


  1.  A type in the `extends` or `implements` clause of a class declaration ([§8.1.4](ch08-classes.md#jls-8.1.4), [§8.1.5](ch08-classes.md#jls-8.1.5))

  2.  A type in the `extends` clause of an interface declaration ([§9.1.3](ch09-interfaces.md#jls-9.1.3))

  3.  The return type of a method ([§8.4.5](ch08-classes.md#jls-8.4.5), [§9.4](ch09-interfaces.md#jls-9.4)), including the type of an element of an annotation interface ([§9.6.1](ch09-interfaces.md#jls-9.6.1))

  4.  A type in the `throws` clause of a method or constructor ([§8.4.6](ch08-classes.md#jls-8.4.6), [§8.8.5](ch08-classes.md#jls-8.8.5), [§9.4](ch09-interfaces.md#jls-9.4))

  5.  A type in the `extends` clause of a type parameter declaration of a generic class, interface, method, or constructor ([§8.1.2](ch08-classes.md#jls-8.1.2), [§9.1.2](ch09-interfaces.md#jls-9.1.2), [§8.4.4](ch08-classes.md#jls-8.4.4), [§8.8.4](ch08-classes.md#jls-8.8.4))

  6.  The type in a field declaration of a class or interface ([§8.3](ch08-classes.md#jls-8.3), [§9.3](ch09-interfaces.md#jls-9.3)), including an enum constant ([§8.9.1](ch08-classes.md#jls-8.9.1))

  7.  The type in a formal parameter declaration of a method, constructor, or lambda expression ([§8.4.1](ch08-classes.md#jls-8.4.1), [§8.8.1](ch08-classes.md#jls-8.8.1), [§9.4](ch09-interfaces.md#jls-9.4), [§15.27.1](ch15-expressions.md#jls-15.27.1))

  8.  The type of the receiver parameter of a method ([§8.4](ch08-classes.md#jls-8.4))

  9.  The type in a local variable declaration in either a statement ([§14.4.2](ch14-blocks-statements-patterns.md#jls-14.4.2), [§14.14.1](ch14-blocks-statements-patterns.md#jls-14.14.1), [§14.14.2](ch14-blocks-statements-patterns.md#jls-14.14.2), [§14.20.3](ch14-blocks-statements-patterns.md#jls-14.20.3)) or a pattern ([§14.30.1](ch14-blocks-statements-patterns.md#jls-14.30.1))

  10. A type in an exception parameter declaration ([§14.20](ch14-blocks-statements-patterns.md#jls-14.20))

  11. The type in a record component declaration of a record class ([§8.10.1](ch08-classes.md#jls-8.10.1))

  

- In expressions:


  12. A type in the explicit type argument list to a constructor invocation, class instance creation expression, method invocation expression, or method reference expression ([§8.8.7.1](ch08-classes.md#jls-8.8.7.1), [§15.9](ch15-expressions.md#jls-15.9), [§15.12](ch15-expressions.md#jls-15.12), [§15.13](ch15-expressions.md#jls-15.13))

  13. In an unqualified class instance creation expression, as the class type to be instantiated ([§15.9](ch15-expressions.md#jls-15.9)) or as the direct superclass type or direct superinterface type of an anonymous class to be instantiated ([§15.9.5](ch15-expressions.md#jls-15.9.5))

  14. The element type in an array creation expression ([§15.10.1](ch15-expressions.md#jls-15.10.1))

  15. The type in the cast operator of a cast expression ([§15.16](ch15-expressions.md#jls-15.16))

  16. The type that follows the `instanceof` type comparison operator ([§15.20.2](ch15-expressions.md#jls-15.20.2))

  17. In a method reference expression ([§15.13](ch15-expressions.md#jls-15.13)), as the reference type to search for a member method or as the class type or array type to construct.

  


Also, types are used as:


- The element type of an array type in any of the above contexts; and

- A non-wildcard type argument, or a bound of a wildcard type argument, of a parameterized type in any of the above contexts.


Finally, there are three special terms in the Java programming language which denote the use of a type:


- An unbounded wildcard ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1))

- The `...` in the type of a variable arity parameter ([§8.4.1](ch08-classes.md#jls-8.4.1)), to indicate an array type

- The simple name of a type in a constructor declaration ([§8.8](ch08-classes.md#jls-8.8)), to indicate the class of the constructed object


The meaning of types in type contexts is given by:


- [§4.2](ch04-types-values-variables.md#jls-4.2), for primitive types

- [§4.4](ch04-types-values-variables.md#jls-4.4), for type parameters

- [§4.5](ch04-types-values-variables.md#jls-4.5), for class and interface types that are parameterized, or appear either as type arguments in a parameterized type or as bounds of wildcard type arguments in a parameterized type

- [§4.8](ch04-types-values-variables.md#jls-4.8), for class and interface types that are raw

- [§4.9](ch04-types-values-variables.md#jls-4.9), for intersection types in the bounds of type parameters

- [§6.5](ch06-names.md#jls-6.5), for types of non-generic classes, interfaces, and type variables

- [§10.1](ch10-arrays.md#jls-10.1), for array types


Some type contexts restrict how a reference type may be parameterized:


- The following type contexts require that if a type is a parameterized reference type, it has no wildcard type arguments:


  - In an `extends` or `implements` clause of a class declaration ([§8.1.4](ch08-classes.md#jls-8.1.4), [§8.1.5](ch08-classes.md#jls-8.1.5))

  - In an `extends` clause of an interface declaration ([§9.1.3](ch09-interfaces.md#jls-9.1.3))

  - In an unqualified class instance creation expression, as the class type to be instantiated ([§15.9](ch15-expressions.md#jls-15.9)) or as the direct superclass type or direct superinterface type of an anonymous class to be instantiated ([§15.9.5](ch15-expressions.md#jls-15.9.5))

  - In a method reference expression ([§15.13](ch15-expressions.md#jls-15.13)), as the reference type to search for a member method or as the class type or array type to construct.

  

  In addition, no wildcard type arguments are permitted in the explicit type argument list to a constructor invocation or class instance creation expression or method invocation expression or method reference expression ([§8.8.7.1](ch08-classes.md#jls-8.8.7.1), [§15.9](ch15-expressions.md#jls-15.9), [§15.12](ch15-expressions.md#jls-15.12), [§15.13](ch15-expressions.md#jls-15.13)).

- The following type contexts require that if a type is a parameterized reference type, it has only unbounded wildcard type arguments (i.e. it is a reifiable type) :


  - As the element type in an array creation expression ([§15.10.1](ch15-expressions.md#jls-15.10.1))

  - As the type that follows the `instanceof` relational operator ([§15.20.2](ch15-expressions.md#jls-15.20.2))

  

- The following type contexts disallow a parameterized reference type altogether, because they involve exceptions and the type of an exception is non-generic ([§6.1](ch06-names.md#jls-6.1)):


  - As the type of an exception that can be thrown by a method or constructor ([§8.4.6](ch08-classes.md#jls-8.4.6), [§8.8.5](ch08-classes.md#jls-8.8.5), [§9.4](ch09-interfaces.md#jls-9.4))

  - In an exception parameter declaration ([§14.20](ch14-blocks-statements-patterns.md#jls-14.20))

  


In any type context where a type is used, it is possible to annotate the keyword denoting a primitive type or the *Identifier* denoting the simple name of a reference type. It is also possible to annotate an array type by writing an annotation to the left of the `[` at the desired level of nesting in the array type. Annotations in these locations are called *type annotations*, and are specified in [§9.7.4](ch09-interfaces.md#jls-9.7.4). Here are some examples:


- `@Foo int[] f;` annotates the primitive type `int`

- `int @Foo [] f;` annotates the array type `int``[]`

- `int @Foo [][] f;` annotates the array type `int``[]``[]`

- `int[] @Foo [] f;` annotates the array type `int``[]` which is the component type of the array type `int``[]``[]`


Some of the *type contexts* which appear in declarations occupy the same syntactic real estate as a number of *declaration contexts* ([§9.6.4.1](ch09-interfaces.md#jls-9.6.4.1)):


- The return type of a method (including the type of an element of an annotation interface)

- The type in a field declaration of a class or interface (including an enum constant)

- The type in a formal parameter declaration of a method, constructor, or lambda expression

- The type in a local variable declaration

- The type in an exception parameter declaration

- The type in a record component declaration of a record class


The fact that the same syntactic location in a program can be both a type context and a declaration context arises because the modifiers for a declaration immediately precede the type of the declared entity. [§9.7.4](ch09-interfaces.md#jls-9.7.4) explains how an annotation in such a location is deemed to appear in a type context or a declaration context or both.


**Example 4.11-1. Usage of a Type**


``` programlisting

import java.util.ArrayList;
import java.util.Collection;
import java.util.Random;

class MiscMath<T extends Number> {
    int divisor;
    MiscMath(int divisor) { this.divisor = divisor; }
    float ratio(long l) {
        try {
            l /= divisor;
        } catch (Exception e) {
            if (e instanceof ArithmeticException)
                l = Long.MAX_VALUE;
            else
                l = 0;
        }
        return (float)l;
    }
    double gausser() {
        Random r = new Random();
        double[] val = new double[2];
        val[0] = r.nextGaussian();
        val[1] = r.nextGaussian();
        return (val[0] + val[1]) / 2;
    }
    Collection<Number> fromArray(Number[] na) {
        Collection<Number> cn = new ArrayList<Number>();
        for (Number n : na) cn.add(n);
        return cn;
    }
    <S> void loop(S s) { this.<S>loop(s); }
}
```

In this example, types are used in declarations of the following:


- Fields, which are the class variables and instance variables of classes ([§8.3](ch08-classes.md#jls-8.3)), and constants of interfaces ([§9.3](ch09-interfaces.md#jls-9.3)); here the field `divisor` in the class `MiscMath` is declared to be of type `int`

- Method parameters ([§8.4.1](ch08-classes.md#jls-8.4.1)); here the parameter `l` of the method `ratio` is declared to be of type `long`

- Method results ([§8.4](ch08-classes.md#jls-8.4)); here the result of the method `ratio` is declared to be of type `float`, and the result of the method `gausser` is declared to be of type `double`

- Constructor parameters ([§8.8.1](ch08-classes.md#jls-8.8.1)); here the parameter of the constructor for `MiscMath` is declared to be of type `int`

- Local variables ([§14.4](ch14-blocks-statements-patterns.md#jls-14.4), [§14.14](ch14-blocks-statements-patterns.md#jls-14.14)); the local variables `r` and `val` of the method `gausser` are declared to be of types `Random` and `double``[]` (array of `double`)

- Exception parameters ([§14.20](ch14-blocks-statements-patterns.md#jls-14.20)); here the exception parameter `e` of the `catch` clause is declared to be of type `Exception`

- Type parameters ([§4.4](ch04-types-values-variables.md#jls-4.4)); here the type parameter of `MiscMath` is a type variable `T` with the type `Number` as its declared bound

- In any declaration that uses a parameterized type; here the type `Number` is used as a type argument ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)) in the parameterized type `Collection<Number>`.


and in expressions of the following kinds:


- Class instance creations ([§15.9](ch15-expressions.md#jls-15.9)); here a local variable `r` of method `gausser` is initialized by a class instance creation expression that uses the type `Random`

- Generic class ([§8.1.2](ch08-classes.md#jls-8.1.2)) instance creations ([§15.9](ch15-expressions.md#jls-15.9)); here `Number` is used as a type argument in the expression `new ArrayList<Number>()`

- Array creations ([§15.10.1](ch15-expressions.md#jls-15.10.1)); here the local variable `val` of method `gausser` is initialized by an array creation expression that creates an array of `double` with size 2

- Generic method ([§8.4.4](ch08-classes.md#jls-8.4.4)) or constructor ([§8.8.4](ch08-classes.md#jls-8.8.4)) invocations ([§15.12](ch15-expressions.md#jls-15.12)); here the method `loop` calls itself with an explicit type argument `S`

- Casts ([§15.16](ch15-expressions.md#jls-15.16)); here the `return` statement of the method `ratio` uses the `float` type in a cast

- The `instanceof` operator ([§15.20.2](ch15-expressions.md#jls-15.20.2)); here the `instanceof` operator tests whether `e` is assignment-compatible with the type `ArithmeticException`


  


## 4.12. Variables


A variable is a storage location and has an associated type, sometimes called its *compile-time type*, that is either a primitive type ([§4.2](ch04-types-values-variables.md#jls-4.2)) or a reference type ([§4.3](ch04-types-values-variables.md#jls-4.3)).

A variable's value is changed by an assignment ([§15.26](ch15-expressions.md#jls-15.26)) or by a prefix or postfix `++` (increment) or `--` (decrement) operator ([§15.14.2](ch15-expressions.md#jls-15.14.2), [§15.14.3](ch15-expressions.md#jls-15.14.3), [§15.15.1](ch15-expressions.md#jls-15.15.1), [§15.15.2](ch15-expressions.md#jls-15.15.2)).

Compatibility of the value of a variable with its type is guaranteed by the design of the Java programming language, as long as a program does not give rise to compile-time unchecked warnings ([§4.12.2](ch04-types-values-variables.md#jls-4.12.2)). Default values ([§4.12.5](ch04-types-values-variables.md#jls-4.12.5)) are compatible and all assignments to a variable are checked for assignment compatibility ([§5.2](ch05-conversions-contexts.md#jls-5.2)), usually at compile time, but, in a single case involving arrays, a run-time check is made ([§10.5](ch10-arrays.md#jls-10.5)).


### 4.12.1. Variables of Primitive Type


A variable of a primitive type always holds a primitive value of that exact primitive type.


### 4.12.2. Variables of Reference Type


A variable of a class type T can hold a null reference or a reference to an instance of class T or of any class that is a subclass of T.

A variable of an interface type can hold a null reference or a reference to any instance of any class that implements the interface.

Note that a variable is not guaranteed to always refer to a subtype of its declared type, but only to subclasses or subinterfaces of the declared type. This is due to the possibility of heap pollution discussed below.

If T is a primitive type, then a variable of type "array of T" can hold a null reference or a reference to any array of type "array of T".

If T is a reference type, then a variable of type "array of T" can hold a null reference or a reference to any array of type "array of S" such that type S is a subclass or subinterface of type T.

A variable of type `Object``[]` can hold a reference to an array of any reference type.

A variable of type `Object` can hold a null reference or a reference to any object, whether it is an instance of a class or an array.

It is possible that a variable of a parameterized type will refer to an object that is not of that parameterized type. This situation is known as *heap pollution*.

Heap pollution can only occur if the program performed some operation involving a raw type that would give rise to a compile-time unchecked warning ([§4.8](ch04-types-values-variables.md#jls-4.8), [§5.1.6](ch05-conversions-contexts.md#jls-5.1.6), [§5.1.9](ch05-conversions-contexts.md#jls-5.1.9), [§8.4.1](ch08-classes.md#jls-8.4.1), [§8.4.8.3](ch08-classes.md#jls-8.4.8.3), [§8.4.8.4](ch08-classes.md#jls-8.4.8.4), [§9.4.1.2](ch09-interfaces.md#jls-9.4.1.2), [§15.12.4.2](ch15-expressions.md#jls-15.12.4.2)), or if the program aliases an array variable of non-reifiable element type through an array variable of a supertype which is either raw or non-generic.


For example, the code:

``` programlisting

List l = new ArrayList<Number>();
List<String> ls = l;  // Unchecked warning
```

gives rise to a compile-time unchecked warning, because it is not possible to ascertain, either at compile time (within the limits of the compile-time type checking rules) or at run time, whether the variable `l` does indeed refer to a `List<String>`.

If the code above is executed, heap pollution arises, as the variable `ls`, declared to be a `List<String>`, refers to a value that is not in fact a `List<String>`.

The problem cannot be identified at run time because type variables are not reified, and thus instances do not carry any information at run time regarding the type arguments used to create them.

In a simple example as given above, it may appear that it should be straightforward to identify the situation at compile time and give an error. However, in the general (and typical) case, the value of the variable `l` may be the result of an invocation of a separately compiled method, or its value may depend upon arbitrary control flow. The code above is therefore very atypical, and indeed very bad style.

Furthermore, the fact that `Object``[]` is a supertype of all array types means that unsafe aliasing can occur which leads to heap pollution. For example, the following code compiles because it is statically type-correct:

``` programlisting


static void m(List<String>... stringLists) {
    Object[] array = stringLists;
    List<Integer> tmpList = Arrays.asList(42);
    array[0] = tmpList;                // (1)
    String s = stringLists[0].get(0);  // (2)
}
```

Heap pollution occurs at (1) because a component in the `stringLists` array that should refer to a `List<String>` now refers to a `List<Integer>`. There is no way to detect this pollution in the presence of both a universal supertype (`Object``[]`) and a non-reifiable type (the declared type of the formal parameter, `List<String>``[]`). No unchecked warning is justified at (1); nevertheless, at run time, a `ClassCastException` will occur at (2).

A compile-time unchecked warning will be given at any invocation of the method above because an invocation is considered by the Java programming language's static type system to create an array whose element type, `List<String>`, is non-reifiable ([§15.12.4.2](ch15-expressions.md#jls-15.12.4.2)). *If and only if* the body of the method was type-safe with respect to the variable arity parameter, then the programmer could use the `SafeVarargs` annotation to silence warnings at invocations ([§9.6.4.7](ch09-interfaces.md#jls-9.6.4.7)). Since the body of the method as written above causes heap pollution, it would be completely inappropriate to use the annotation to disable warnings for callers.

Finally, note that the `stringLists` array could be aliased through variables of types other than `Object``[]`, and heap pollution could still occur. For example, the type of the `array` variable could be `java.util.Collection[]` - a raw element type - and the body of the method above would compile without warnings or errors and still cause heap pollution. And if the Java SE Platform defined, say, `Sequence` as a non-generic supertype of `List<T>`, then using `Sequence` as the type of `array` would also cause heap pollution.


The variable will always refer to an object that is an instance of a class that represents the parameterized type.

The value of `ls` in the example above is always an instance of a class that provides a representation of a `List`.

Assignment from an expression of a raw type to a variable of a parameterized type should only be used when combining legacy code which does not make use of parameterized types with more modern code that does.

If no operation that requires a compile-time unchecked warning to be issued takes place, and no unsafe aliasing occurs of array variables with non-reifiable element types, then heap pollution cannot occur. Note that this does not imply that heap pollution only occurs if a compile-time unchecked warning actually occurred. It is possible to run a program where some of the binaries were produced by a compiler for an older version of the Java programming language, or from sources that explicitly suppressed unchecked warnings. This practice is unhealthy at best.

Conversely, it is possible that despite executing code that could (and perhaps did) give rise to a compile-time unchecked warning, no heap pollution takes place. Indeed, good programming practice requires that the programmer satisfy herself that despite any unchecked warning, the code is correct and heap pollution will not occur.


### 4.12.3. Kinds of Variables


There are eight kinds of variables:


1.  A *class variable* is a field declared using the keyword `static` within a class declaration ([§8.3.1.1](ch08-classes.md#jls-8.3.1.1)), or with or without the keyword `static` within an interface declaration ([§9.3](ch09-interfaces.md#jls-9.3)).

    A class variable is created when its class or interface is prepared ([§12.3.2](ch12-execution.md#jls-12.3.2)) and is initialized to a default value ([§4.12.5](ch04-types-values-variables.md#jls-4.12.5)). The class variable effectively ceases to exist when its class or interface is unloaded ([§12.7](ch12-execution.md#jls-12.7)).

2.  An *instance variable* is a field declared within a class declaration without using the keyword `static` ([§8.3.1.1](ch08-classes.md#jls-8.3.1.1)).

    If a class T has a field `a` that is an instance variable, then a new instance variable `a` is created and initialized to a default value ([§4.12.5](ch04-types-values-variables.md#jls-4.12.5)) as part of each newly created object of class T or of any class that is a subclass of T ([§8.1.4](ch08-classes.md#jls-8.1.4)). The instance variable effectively ceases to exist when the object of which it is a field is no longer referenced, after any necessary finalization of the object ([§12.6](ch12-execution.md#jls-12.6)) has been completed.

3.  *Array components* are unnamed variables that are created and initialized to default values ([§4.12.5](ch04-types-values-variables.md#jls-4.12.5)) whenever a new object that is an array is created ([§10 (Arrays)](ch10-arrays.md), [§15.10.2](ch15-expressions.md#jls-15.10.2)). The array components effectively cease to exist when the array is no longer referenced.

4.  *Method parameters* ([§8.4.1](ch08-classes.md#jls-8.4.1)) name argument values passed to a method.

    For every parameter declared in a method declaration, a new parameter variable is created each time that method is invoked ([§15.12](ch15-expressions.md#jls-15.12)). The new variable is initialized with the corresponding argument value from the method invocation. The method parameter effectively ceases to exist when the execution of the body of the method is complete.

5.  *Constructor parameters* ([§8.8.1](ch08-classes.md#jls-8.8.1)) name argument values passed to a constructor.

    For every parameter declared in a constructor declaration, a new parameter variable is created each time a class instance creation expression ([§15.9](ch15-expressions.md#jls-15.9)) or constructor invocation ([§8.8.7](ch08-classes.md#jls-8.8.7)) invokes that constructor. The new variable is initialized with the corresponding argument value from the creation expression or constructor invocation. The constructor parameter effectively ceases to exist when the execution of the body of the constructor is complete.

6.  *Lambda parameters* ([§15.27.1](ch15-expressions.md#jls-15.27.1)) name argument values passed to a lambda expression body ([§15.27.2](ch15-expressions.md#jls-15.27.2)).

    For every parameter declared in a lambda expression, a new parameter variable is created each time a method implemented by the lambda body is invoked ([§15.12](ch15-expressions.md#jls-15.12)). The new variable is initialized with the corresponding argument value from the method invocation. The lambda parameter effectively ceases to exist when the execution of the lambda expression body is complete.

7.  An *exception parameter* is created each time an exception is caught by a `catch` clause of a `try` statement ([§14.20](ch14-blocks-statements-patterns.md#jls-14.20)).

    The new variable is initialized with the actual object associated with the exception ([§11.3](ch11-exceptions.md#jls-11.3), [§14.18](ch14-blocks-statements-patterns.md#jls-14.18)). The exception parameter effectively ceases to exist when execution of the block associated with the `catch` clause is complete.

8.  *Local variables* ([§14.4](ch14-blocks-statements-patterns.md#jls-14.4)) are declared by statements ([§14.4.2](ch14-blocks-statements-patterns.md#jls-14.4.2), [§14.14.1](ch14-blocks-statements-patterns.md#jls-14.14.1), [§14.14.2](ch14-blocks-statements-patterns.md#jls-14.14.2), [§14.20.3](ch14-blocks-statements-patterns.md#jls-14.20.3)) and by patterns ([§14.30](ch14-blocks-statements-patterns.md#jls-14.30)). A local variable declared by a pattern is called a *pattern variable*.

    A local variable declared by a statement is created when the flow of control enters the nearest enclosing block ([§14.2](ch14-blocks-statements-patterns.md#jls-14.2)), `for` statement, or `try`-with-resources statement.

    A local variable declared by a statement is initialized as part of the execution of the statement, provided the variable's declarator has an initializer. The rules of definite assignment ([§16 (Definite Assignment)](ch16-definite-assignment.md)) prevent the value of a local variable declared by a statement from being used before it has been initialized or otherwise assigned a value.

    A local variable declared by a pattern is created and initialized when the pattern matches ([§14.30.2](ch14-blocks-statements-patterns.md#jls-14.30.2)). The rules of scoping ([§6.3](ch06-names.md#jls-6.3)) prevent the value of a local variable declared by a pattern from being used unless the pattern has matched.

    A local variable ceases to exist when its declaration is no longer in scope.

    Were it not for one exceptional situation, a local variable declared by a statement could always be regarded as being created when the statement is executed. The exceptional situation involves the `switch` statement ([§14.11](ch14-blocks-statements-patterns.md#jls-14.11)), where it is possible for control to enter a block but bypass execution of a local variable declaration statement. Because of the restrictions imposed by the rules of definite assignment ([§16 (Definite Assignment)](ch16-definite-assignment.md)), however, the local variable declared by such a bypassed local variable declaration statement cannot be used before it has been definitely assigned a value by an assignment expression ([§15.26](ch15-expressions.md#jls-15.26)).


**Example 4.12.3-1. Different Kinds of Variables**


``` programlisting

class Point {
    static int numPoints;   // numPoints is a class variable
    int x, y;               // x and y are instance variables
    int[] w = new int[10];  // w[0] is an array component
    int setX(int x) {       // x is a method parameter
        int oldx = this.x;  // oldx is a local variable
        this.x = x;
        return oldx;
    }
    boolean equalAtX(Object o) {
        if (o instanceof Point p)  // p is a pattern variable
            return this.x == p.x;
        else
            return false;
    }
}
```


  


### 4.12.4. `final` Variables


A variable can be declared `final`. A `final` variable may only be assigned to once. It is a compile-time error if a `final` variable is assigned to unless it is definitely unassigned immediately prior to the assignment ([§16 (Definite Assignment)](ch16-definite-assignment.md)).

Once a `final` variable has been assigned, it always contains the same value. If a `final` variable holds a reference to an object, then the state of the object may be changed by operations on the object, but the variable will always refer to the same object. This applies also to arrays, because arrays are objects; if a `final` variable holds a reference to an array, then the components of the array may be changed by operations on the array, but the variable will always refer to the same array.

A *blank `final`* is a `final` variable whose declaration lacks an initializer.

A *constant variable* is a `final` variable of primitive type or type `String` that is initialized with a constant expression ([§15.29](ch15-expressions.md#jls-15.29)). Whether a variable is a constant variable or not may have implications with respect to class initialization ([§12.4.1](ch12-execution.md#jls-12.4.1)), binary compatibility ([§13.1](ch13-binary-compatibility.md#jls-13.1)), reachability ([§14.22](ch14-blocks-statements-patterns.md#jls-14.22)), and definite assignment ([§16.1.1](ch16-definite-assignment.md#jls-16.1.1)).

Three kinds of variable are implicitly declared `final`: a field of an interface ([§9.3](ch09-interfaces.md#jls-9.3)), a local variable declared as a resource of a `try`-with-resources statement ([§14.20.3](ch14-blocks-statements-patterns.md#jls-14.20.3)), and an exception parameter of a multi-`catch` clause ([§14.20](ch14-blocks-statements-patterns.md#jls-14.20)). An exception parameter of a uni-`catch` clause is never implicitly declared `final`, but may be effectively final.


**Example 4.12.4-1. Final Variables**


Declaring a variable `final` can serve as useful documentation that its value will not change and can help avoid programming errors. In this program:

``` programlisting

class Point {
    int x, y;
    int useCount;
    Point(int x, int y) { this.x = x; this.y = y; }
    static final Point origin = new Point(0, 0);
}
```

the class `Point` declares a `final` class variable `origin`. The `origin` variable holds a reference to an object that is an instance of class `Point` whose coordinates are (0, 0). The value of the variable `Point.origin` can never change, so it always refers to the same `Point` object, the one created by its initializer. However, an operation on this `Point` object might change its state - for example, modifying its `useCount` or even, misleadingly, its `x` or `y` coordinate.


  

Certain variables that are not declared `final` are instead considered *effectively final*:


- A local variable declared by a statement and whose declarator has an initializer ([§14.4](ch14-blocks-statements-patterns.md#jls-14.4)), or a local variable declared by a pattern ([§14.30.1](ch14-blocks-statements-patterns.md#jls-14.30.1)), is *effectively final* if all of the following are true:


  - It is not declared `final`.

  - It never occurs as the left hand side in an assignment expression ([§15.26](ch15-expressions.md#jls-15.26)). (Note that the local variable declarator containing the initializer is *not* an assignment expression.)

  - It never occurs as the operand of a prefix or postfix increment or decrement operator ([§15.14](ch15-expressions.md#jls-15.14), [§15.15](ch15-expressions.md#jls-15.15)).

  

- A local variable declared by a statement and whose declarator lacks an initializer is *effectively final* if all of the following are true:


  - It is not declared `final`.

  - Whenever it occurs as the left hand side in an assignment expression, it is definitely unassigned and not definitely assigned before the assignment; that is, it is definitely unassigned and not definitely assigned after the right hand side of the assignment expression ([§16 (Definite Assignment)](ch16-definite-assignment.md)).

  - It never occurs as the operand of a prefix or postfix increment or decrement operator.

  

- A method, constructor, lambda, or exception parameter ([§8.4.1](ch08-classes.md#jls-8.4.1), [§8.8.1](ch08-classes.md#jls-8.8.1), [§9.4](ch09-interfaces.md#jls-9.4), [§15.27.1](ch15-expressions.md#jls-15.27.1), [§14.20](ch14-blocks-statements-patterns.md#jls-14.20)) is treated, for the purpose of determining whether it is *effectively final*, as a local variable whose declarator has an initializer.


If a variable is effectively final, adding the `final` modifier to its declaration will not introduce any compile-time errors. Conversely, a local variable or parameter that is declared `final` in a valid program becomes effectively final if the `final` modifier is removed.


### 4.12.5. Initial Values of Variables


Every variable in a program must have a value before its value is used:


- Each class variable, instance variable, or array component is initialized with a *default value* when it is created ([§15.9](ch15-expressions.md#jls-15.9), [§15.10.2](ch15-expressions.md#jls-15.10.2)):


  - For type `byte`, the default value is zero, that is, the value of `(byte)0`.

  - For type `short`, the default value is zero, that is, the value of `(short)0`.

  - For type `int`, the default value is zero, that is, `0`.

  - For type `long`, the default value is zero, that is, `0L`.

  - For type `float`, the default value is positive zero, that is, `0.0f`.

  - For type `double`, the default value is positive zero, that is, `0.0d`.

  - For type `char`, the default value is the null character, that is, `'\u0000'`.

  - For type `boolean`, the default value is `false`.

  - For all reference types ([§4.3](ch04-types-values-variables.md#jls-4.3)), the default value is `null`.

  

- Each method parameter ([§8.4.1](ch08-classes.md#jls-8.4.1)) is initialized to the corresponding argument value provided by the invoker of the method ([§15.12](ch15-expressions.md#jls-15.12)).

- Each constructor parameter ([§8.8.1](ch08-classes.md#jls-8.8.1)) is initialized to the corresponding argument value provided by a class instance creation expression ([§15.9](ch15-expressions.md#jls-15.9)) or constructor invocation ([§8.8.7](ch08-classes.md#jls-8.8.7)).

- An exception parameter ([§14.20](ch14-blocks-statements-patterns.md#jls-14.20)) is initialized to the thrown object representing the exception ([§11.3](ch11-exceptions.md#jls-11.3), [§14.18](ch14-blocks-statements-patterns.md#jls-14.18)).

- A local variable declared by a statement ([§14.4.2](ch14-blocks-statements-patterns.md#jls-14.4.2), [§14.14.1](ch14-blocks-statements-patterns.md#jls-14.14.1), [§14.14.2](ch14-blocks-statements-patterns.md#jls-14.14.2), [§14.20.3](ch14-blocks-statements-patterns.md#jls-14.20.3)) must be explicitly given a value before it is used, by either initialization ([§14.4](ch14-blocks-statements-patterns.md#jls-14.4)) or assignment ([§15.26](ch15-expressions.md#jls-15.26)), in a way that can be verified using the rules for definite assignment ([§16 (Definite Assignment)](ch16-definite-assignment.md)).

  A local variable declared by a pattern ([§14.30.1](ch14-blocks-statements-patterns.md#jls-14.30.1)) is initialized implicitly, by the process of pattern matching ([§14.30.2](ch14-blocks-statements-patterns.md#jls-14.30.2)).


**Example 4.12.5-1. Initial Values of Variables**


``` programlisting

class Point {
    static int npoints;
    int x, y;
    Point root;
}

class Test {
    public static void main(String[] args) {
        System.out.println("npoints=" + Point.npoints);
        Point p = new Point();
        System.out.println("p.x=" + p.x + ", p.y=" + p.y);
        System.out.println("p.root=" + p.root);
    }
}
```

This program prints:

``` programlisting

npoints=0
p.x=0, p.y=0
p.root=null
```

illustrating the default initialization of `npoints`, which occurs when the class `Point` is prepared ([§12.3.2](ch12-execution.md#jls-12.3.2)), and the default initialization of `x`, `y`, and `root`, which occurs when a new `Point` is instantiated. See [§12 (Execution)](ch12-execution.md) for a full description of all aspects of loading, linking, and initialization of classes and interfaces, plus a description of the instantiation of classes to make new class instances.


  


### 4.12.6. Types, Classes, and Interfaces


In the Java programming language, every variable and every expression has a type that can be determined at compile time. The type may be a primitive type or a reference type. Reference types include class types and interface types. Reference types are introduced by *type declarations*, which include class declarations ([§8.1](ch08-classes.md#jls-8.1)) and interface declarations ([§9.1](ch09-interfaces.md#jls-9.1)). We often use the term *type* to refer to either a class or an interface.

In the Java Virtual Machine, every object belongs to some particular class: the class that was mentioned in the creation expression that produced the object ([§15.9](ch15-expressions.md#jls-15.9)), or the class whose `Class` object was used to invoke a reflective method to produce the object, or the `String` class for objects implicitly created by the string concatenation operator `+` ([§15.18.1](ch15-expressions.md#jls-15.18.1)). This class is called the *class of the object*. An object is said to be an *instance* of its class and of all superclasses of its class.

Every array also has a class. The method `getClass`, when invoked for an array object, will return a class object (of class `Class`) that represents the *class of the array* ([§10.8](ch10-arrays.md#jls-10.8)).

The compile-time type of a variable is always declared, and the compile-time type of an expression can be deduced at compile time. The compile-time type limits the possible values that the variable can hold at run time or the expression can produce at run time. If a run-time value is a reference that is not `null`, it refers to an object or array that has a class, and that class will necessarily be compatible with the compile-time type.

Even though a variable or expression may have a compile-time type that is an interface type, there are no instances of interfaces. A variable or expression whose type is an interface type can reference any object whose class implements ([§8.1.5](ch08-classes.md#jls-8.1.5)) that interface.

Sometimes a variable or expression is said to have a "run-time type". This refers to the class of the object referred to by the value of the variable or expression at run time, assuming that the value is not `null`.

The correspondence between compile-time types and run-time types is incomplete for two reasons:


1.  At run time, classes and interfaces are loaded by the Java Virtual Machine using class loaders. Each class loader defines its own set of classes and interfaces. As a result, it is possible for two loaders to load an identical class or interface definition but produce distinct classes or interfaces at run time. Consequently, code that compiled correctly may fail at link time if the class loaders that load it are inconsistent.

    See the paper *Dynamic Class Loading in the Java Virtual Machine*, by Sheng Liang and Gilad Bracha, in *Proceedings of OOPSLA '98*, published as *ACM SIGPLAN Notices*, Volume 33, Number 10, October 1998, pages 36-44, and *The Java Virtual Machine Specification, Java SE 26 Edition* for more details.

2.  Type variables ([§4.4](ch04-types-values-variables.md#jls-4.4)) and type arguments ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)) are not reified at run time. As a result, the same class or interface at run time represents multiple parameterized types ([§4.5](ch04-types-values-variables.md#jls-4.5)) from compile time. Specifically, all compile-time parameterizations of a given generic type ([§8.1.2](ch08-classes.md#jls-8.1.2), [§9.1.2](ch09-interfaces.md#jls-9.1.2)) share a single run-time representation.

    Under certain conditions, it is possible that a variable of a parameterized type refers to an object that is not of that parameterized type. This situation is known as *heap pollution* ([§4.12.2](ch04-types-values-variables.md#jls-4.12.2)). The variable will always refer to an object that is an instance of a class that represents the parameterized type.


**Example 4.12.6-1. Type of a Variable versus Class of an Object**


``` programlisting

interface Colorable {
    void setColor(byte r, byte g, byte b);
}

class Point { int x, y; }

class ColoredPoint extends Point implements Colorable {
    byte r, g, b;
    public void setColor(byte rv, byte gv, byte bv) {
        r = rv; g = gv; b = bv;
    }
}

class Test {
    public static void main(String[] args) {
        Point p = new Point();
        ColoredPoint cp = new ColoredPoint();
        p = cp;
        Colorable c = cp;
    }
}
```

In this example:


- The local variable `p` of the method `main` of class `Test` has type `Point` and is initially assigned a reference to a new instance of class `Point`.

- The local variable `cp` similarly has as its type `ColoredPoint`, and is initially assigned a reference to a new instance of class `ColoredPoint`.

- The assignment of the value of `cp` to the variable `p` causes `p` to hold a reference to a `ColoredPoint` object. This is permitted because `ColoredPoint` is a subclass of `Point`, so the class `ColoredPoint` is assignment-compatible ([§5.2](ch05-conversions-contexts.md#jls-5.2)) with the type `Point`. A `ColoredPoint` object includes support for all the methods of a `Point`. In addition to its particular fields `r`, `g`, and `b`, it has the fields of class `Point`, namely `x` and `y`.

- The local variable `c` has as its type the interface type `Colorable`, so it can hold a reference to any object whose class implements `Colorable`; specifically, it can hold a reference to a `ColoredPoint`.


Note that an expression such as `new Colorable()` is not valid because it is not possible to create an instance of an interface, only of a class. However, the expression `new Colorable() { public void setColor... }` is valid because it declares an anonymous class ([§15.9.5](ch15-expressions.md#jls-15.9.5)) that implements the `Colorable` interface.


  


