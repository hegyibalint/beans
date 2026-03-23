# Chapter 18. Type Inference


## Contents

[18.1. Concepts and Notation](ch18-type-inference.md#jls-18.1)

[18.1.1. Inference Variables](ch18-type-inference.md#jls-18.1.1)

[18.1.2. Constraint Formulas](ch18-type-inference.md#jls-18.1.2)

[18.1.3. Bounds](ch18-type-inference.md#jls-18.1.3)

[18.2. Reduction](ch18-type-inference.md#jls-18.2)

[18.2.1. Expression Compatibility Constraints](ch18-type-inference.md#jls-18.2.1)

[18.2.2. Type Compatibility Constraints](ch18-type-inference.md#jls-18.2.2)

[18.2.3. Subtyping Constraints](ch18-type-inference.md#jls-18.2.3)

[18.2.4. Type Equality Constraints](ch18-type-inference.md#jls-18.2.4)

[18.2.5. Checked Exception Constraints](ch18-type-inference.md#jls-18.2.5)

[18.3. Incorporation](ch18-type-inference.md#jls-18.3)

[18.3.1. Complementary Pairs of Bounds](ch18-type-inference.md#jls-18.3.1)

[18.3.2. Bounds Involving Capture Conversion](ch18-type-inference.md#jls-18.3.2)

[18.4. Resolution](ch18-type-inference.md#jls-18.4)

[18.5. Uses of Inference](ch18-type-inference.md#jls-18.5)

[18.5.1. Invocation Applicability Inference](ch18-type-inference.md#jls-18.5.1)

[18.5.2. Invocation Type Inference](ch18-type-inference.md#jls-18.5.2)

[18.5.2.1. Poly Method Invocation Compatibility](ch18-type-inference.md#jls-18.5.2.1)

[18.5.2.2. Additional Argument Constraints](ch18-type-inference.md#jls-18.5.2.2)

[18.5.3. Functional Interface Parameterization Inference](ch18-type-inference.md#jls-18.5.3)

[18.5.4. More Specific Method Inference](ch18-type-inference.md#jls-18.5.4)

[18.5.5. Record Pattern Type Inference](ch18-type-inference.md#jls-18.5.5)


A variety of compile-time analyses require reasoning about types that are not yet known. Principal among these are generic method applicability testing ([§18.5.1](ch18-type-inference.md#jls-18.5.1)) and generic method invocation type inference ([§18.5.2](ch18-type-inference.md#jls-18.5.2)). In general, we refer to the process of reasoning about unknown types as *type inference*.

At a high level, type inference can be decomposed into three processes:


- *Reduction* takes a compatibility assertion about an expression or type, called a *constraint formula*, and reduces it to a set of *bounds* on *inference variables*. Often, a constraint formula reduces to *other* constraint formulas, which must be recursively reduced. A procedure is followed to identify these additional constraint formulas and, ultimately, to express via a bound set the conditions under which the choices for inferred types would render each constraint formula true.

- *Incorporation* maintains a set of inference variable bounds, ensuring that these are consistent as new bounds are added. Because the bounds on one variable can sometimes impact the possible choices for another variable, this process propagates bounds between such interdependent variables.

- *Resolution* examines the bounds on an inference variable and determines an *instantiation* that is compatible with those bounds. It also decides the order in which interdependent inference variables are to be resolved.


These processes interact closely: reduction can trigger incorporation; incorporation may lead to further reduction; and resolution may cause further incorporation.


- [§18.1](ch18-type-inference.md#jls-18.1) more precisely defines the concepts used as intermediate results and the notation used to express them.

- [§18.2](ch18-type-inference.md#jls-18.2) describes reduction in detail.

- [§18.3](ch18-type-inference.md#jls-18.3) describes incorporation in detail.

- [§18.4](ch18-type-inference.md#jls-18.4) describes resolution in detail.

- [§18.5](ch18-type-inference.md#jls-18.5) defines how these inference tools are used to solve certain compile-time analysis problems.


In comparison to the Java SE 7 Edition of *The Java Language Specification*, important changes to inference include:


- Adding support for lambda expressions and method references as method invocation arguments.

- Generalizing to define inference in terms of poly expressions, which may not have well-defined types until *after* inference is complete. This has the notable effect of improving inference for nested generic method and diamond constructor invocations.

- Describing how inference is used to handle wildcard-parameterized functional interface target types and most specific method analysis.

- Clarifying the distinction between invocation applicability testing (which involves only the invocation arguments) and invocation type inference (which incorporates a target type).

- Delaying resolution of all inference variables, even those with lower bounds, until invocation type inference, in order to get better results.

- Improving inference behavior for interdependent (or self-dependent) variables.

- Eliminating bugs and potential sources of confusion. This revision more carefully and precisely handles the distinction between specific conversion contexts and subtyping, and describes reduction by paralleling the corresponding non-inference relations. Where there are intentional departures from the non-inference relations, these are explicitly identified as such.

- Laying a foundation for future evolution: enhancements to or new applications of inference will be easier to integrate into the specification.


## 18.1. Concepts and Notation


This section defines *inference variables*, *constraint formulas*, and *bounds*, as the terms will be used throughout this chapter. It also presents notation.


### 18.1.1. Inference Variables


*Inference variables* are *meta-variables* for types - that is, they are special names that allow abstract reasoning about types. To distinguish them from *type variables*, inference variables are represented with Greek letters, principally α.

The term "type" is used loosely in this chapter to include type-like syntax that contains inference variables. The term *proper type* excludes such "types" that mention inference variables. Assertions that involve inference variables are assertions about every proper type that can be produced by replacing each inference variable with a proper type.


### 18.1.2. Constraint Formulas


*Constraint formulas* are assertions of compatibility or subtyping that may involve inference variables. The formulas may take one of the following forms:


- ‹*Expression* → T›: An expression is compatible in a loose invocation context with type T ([§5.3](ch05-conversions-contexts.md#jls-5.3)).

- ‹S → T›: A type S is compatible in a loose invocation context with type T ([§5.3](ch05-conversions-contexts.md#jls-5.3)).

- ‹S `<:` T›: A reference type S is a subtype of a reference type T ([§4.10](ch04-types-values-variables.md#jls-4.10)).

- ‹S `<=` T›: A type argument S is contained by a type argument T ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)).

- ‹S = T›: A type S is the same as a type T ([§4.3.4](ch04-types-values-variables.md#jls-4.3.4)), or a type argument S is the same as type argument T.

- ‹*LambdaExpression* →<sub>*throws*</sub> T›: The checked exceptions thrown by the body of the *LambdaExpression* are declared by the `throws` clause of the function type derived from T.

- ‹*MethodReference* →<sub>*throws*</sub> T›: The checked exceptions thrown by the referenced method are declared by the `throws` clause of the function type derived from T.


Examples of constraint formulas:


- From `Collections.singleton("hi")`, we have the constraint formula ‹`"hi"` → α›. Through reduction, this will become the constraint formula: ‹`String` `<:` α›.

- From `Arrays.asList(1, 2.0)`, we have the constraint formulas ‹`1` → α› and ‹`2.0` → α›. Through reduction, these will become the constraint formulas ‹`int` → α› and ‹`double` → α›, and then ‹`Integer` `<:` α› and ‹`Double` `<:` α›.

- From the target type of the constructor invocation `List<Thread> lt = new ArrayList<>()`, we have the constraint formula ‹`ArrayList``<`α`>` → `List``<``Thread``>`›. Through reduction, this will become the constraint formula ‹α `<=` `Thread`›, and then ‹α = `Thread`›.


### 18.1.3. Bounds


During the inference process, a set of *bounds* on inference variables is maintained. A bound has one of the following forms:


- S = T, where at least one of S or T is an inference variable: S is the same as T.

- S `<:` T, where at least one of S or T is an inference variable: S is a subtype of T.

- *false*: No valid choice of inference variables exists.

- G`<`α<sub>1</sub>, ..., α<sub>n</sub>`>` = capture(G`<`A<sub>1</sub>, ..., A<sub>n</sub>`>`): The variables α<sub>1</sub>, ..., α<sub>n</sub> represent the result of capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) applied to G`<`A<sub>1</sub>, ..., A<sub>n</sub>`>` (where A<sub>1</sub>, ..., A<sub>n</sub> may be types or wildcards and may mention inference variables).

- `throws` α: The inference variable α appears in a `throws` clause.


A bound is *satisfied* by an inference variable substitution if, after applying the substitution, the assertion is true. The bound *false* can never be satisfied.

Some bounds relate an inference variable to a proper type. Let T be a proper type. Given a bound of the form α = T or T = α, we say T is an *instantiation* of α. Similarly, given a bound of the form α `<:` T, we say T is a *proper upper bound* of α, and given a bound of the form T `<:` α, we say T is a *proper lower bound* of α.

Other bounds relate two inference variables, or an inference variable to a type that contains inference variables. Such bounds, of the form S = T or S `<:` T, are called *dependencies*.

A bound of the form G`<`α<sub>1</sub>, ..., α<sub>n</sub>`>` = capture(G`<`A<sub>1</sub>, ..., A<sub>n</sub>`>`) indicates that α<sub>1</sub>, ..., α<sub>n</sub> are placeholders for the results of capture conversion. This is necessary because capture conversion can only be performed on a proper type, and the inference variables in A<sub>1</sub>, ..., A<sub>n</sub> may not yet be resolved.

A bound of the form `throws` α is purely informational: it directs resolution to optimize the instantiation of α so that, if possible, it is not a checked exception type.

An important intermediate result of inference is a *bound set*. It is sometimes convenient to refer to an *empty* bound set with the symbol *true*; this is merely out of convenience, and the two are interchangeable.


Examples of bound sets:


- { α = `String` } contains a single bound, instantiating α as `String`.

- { `Integer` `<:` α, `Double` `<:` α, α `<:` `Object` } describes two proper lower bounds and one proper upper bound for α.

- { α `<:` `Iterable<?>`, β `<:` `Object`, α `<:` `List``<`β`>` } describes a proper upper bound for each of α and β, along with a dependency between them.

- { } contains no bounds nor dependencies, and can be referred to as *true*.

- { *false* } expresses the fact that no satisfactory instantiation exists.


When inference begins, a bound set is typically generated from a list of type parameter declarations P<sub>1</sub>, ..., P<sub>p</sub> and associated inference variables α<sub>1</sub>, ..., α<sub>p</sub>. Such a bound set is generated as follows. For each *l* (1 ≤ *l* ≤ *p*):


- If P<sub>l</sub> has no *TypeBound*, the bound *α<sub>l</sub> `<:` `Object`* appears in the set.

- Otherwise, for each type T delimited by `&` in the *TypeBound*, the bound α<sub>l</sub> `<:` T`[`P<sub>1</sub>:=α<sub>1</sub>, ..., P<sub>p</sub>:=α<sub>p</sub>`]` appears in the set; if this results in no proper upper bounds for α<sub>l</sub> (only dependencies), then the bound α<sub>l</sub> `<:` `Object` also appears in the set.


## 18.2. Reduction


*Reduction* is the process by which a set of constraint formulas ([§18.1.2](ch18-type-inference.md#jls-18.1.2)) is simplified to produce a bound set ([§18.1.3](ch18-type-inference.md#jls-18.1.3)).

Each constraint formula is considered in turn. The rules in this section specify how the formula is reduced to one or both of:


- A bound or bound set, which is to be incorporated with the "current" bound set. Initially, the current bound set is empty.

- Further constraint formulas, which are to be reduced recursively.


Reduction completes when no further constraint formulas remain to be reduced.

The results of a reduction step are always *soundness-preserving*: if an inference variable instantiation satisfies the reduced constraints and bounds, it will also satisfy the original constraint. On the other hand, reduction is not *completeness-preserving*: there may exist inference variable instantiations that satisfy the original constraint but *do not* satisfy a reduced constraint or bound. This is due to inherent limitations of the algorithm, along with a desire to avoid undue complexity. One effect is that there are expressions for which type argument inference fails to find a solution, but that can be well-typed if the programmer explicitly inserts appropriate types.


### 18.2.1. Expression Compatibility Constraints


A constraint formula of the form ‹*Expression* → T› is reduced as follows:


- If T is a proper type, the constraint reduces to *true* if the expression is compatible in a loose invocation context with T ([§5.3](ch05-conversions-contexts.md#jls-5.3)), and *false* otherwise.

- Otherwise, if the expression is a standalone expression ([§15.2](ch15-expressions.md#jls-15.2)) of type S, the constraint reduces to ‹S → T›.

- Otherwise, the expression is a poly expression ([§15.2](ch15-expressions.md#jls-15.2)). The result depends on the form of the expression:


  - If the expression is a parenthesized expression of the form `(` *Expression*' `)`, the constraint reduces to ‹*Expression*' → T›.

  - If the expression is a class instance creation expression or a method invocation expression, the constraint reduces to the bound set B<sub>3</sub> which would be used to determine the expression's compatibility with target type T, as defined in [§18.5.2.1](ch18-type-inference.md#jls-18.5.2.1). (For a class instance creation expression, the corresponding "method" used for inference is defined in [§15.9.3](ch15-expressions.md#jls-15.9.3).)

    This bound set may contain new inference variables, as well as dependencies between these new variables and the inference variables in T.

  - If the expression is a conditional expression of the form `e`<sub>`1`</sub> `?` `e`<sub>`2`</sub> `:` `e`<sub>`3`</sub>, the constraint reduces to two constraint formulas, ‹`e`<sub>`2`</sub> → T› and ‹`e`<sub>`3`</sub> → T›.

  - If the expression is a lambda expression or a method reference expression, the result is specified below.

  - If the expression is a `switch` expression with result expressions `e`<sub>`1`</sub>, ..., `e`<sub>`n`</sub>, the constraint reduces to *n* constraint formulas, ‹`e`<sub>`1`</sub> → T›, ..., ‹`e`<sub>`n`</sub> → T›.

  


By treating nested generic method invocations as poly expressions, we improve the behavior of inference for nested invocations. For example, the following is illegal in Java SE 7 but legal in Java SE 8:

``` screen

ProcessBuilder b = new ProcessBuilder(Collections.emptyList());
  // ProcessBuilder's constructor expects a List<String>
```

When *both* the outer and the nested invocation require inference, the problem is more difficult. For example:

``` screen
List<String> ls = new ArrayList<>(Collections.emptyList());
```

Our approach is to "lift" the bounds inferred for the nested invocation (simply { α `<:` `Object` } in the case of `emptyList`) into the outer inference process (in this case, trying to infer β where the constructor is for type `ArrayList``<`β`>`). We also infer dependencies between the nested inference variables and the outer inference variables (the constraint ‹`List``<`α`>` → `Collection``<`β`>`› would reduce to the dependency α = β). In this way, resolution of the inference variables in the nested invocation can wait until additional information can be inferred from the outer invocation (based on the assignment target, β = `String`).

A constraint formula of the form ‹*LambdaExpression* → T›, where T mentions at least one inference variable, is reduced as follows:


- If T is not a functional interface type ([§9.8](ch09-interfaces.md#jls-9.8)), the constraint reduces to *false*.

- Otherwise, let T' be the ground target type derived from T, as specified in [§15.27.3](ch15-expressions.md#jls-15.27.3). If [§18.5.3](ch18-type-inference.md#jls-18.5.3) is used to derive a functional interface type which is parameterized, then the test that F`<`A'<sub>1</sub>, ..., A'<sub>m</sub>`>` is a subtype of F`<`A<sub>1</sub>, ..., A<sub>m</sub>`>` is not performed (instead, it is asserted with a constraint formula below). Let the target function type for the lambda expression be the function type of T'. Then:


  - If no valid function type can be found, the constraint reduces to *false*.

  - Otherwise, the congruence of *LambdaExpression* with the target function type is asserted as follows:


    - If the number of lambda parameters differs from the number of parameter types of the function type, the constraint reduces to *false*.

    - If the lambda expression is implicitly typed and one or more of the function type's parameter types is not a proper type, the constraint reduces to *false*.

      This condition never arises in practice, due to the handling of implicitly typed lambda expressions in [§18.5.1](ch18-type-inference.md#jls-18.5.1) and the substitution applied to the target type in [§18.5.2.2](ch18-type-inference.md#jls-18.5.2.2).

    - If the function type's result is `void` and the lambda body is neither a statement expression nor a void-compatible block, the constraint reduces to *false*.

    - If the function type's result is not `void` and the lambda body is a block that is not value-compatible, the constraint reduces to *false*.

    - Otherwise, the constraint reduces to all of the following constraint formulas:


      - If the lambda parameters have explicitly declared types F<sub>1</sub>, ..., F<sub>n</sub> and the function type has parameter types G<sub>1</sub>, ..., G<sub>n</sub>, then (i) for all *i* (1 ≤ *i* ≤ *n*), ‹F<sub>i</sub> = G<sub>i</sub>›, and (ii) ‹T' `<:` T›.

      - If the function type's return type is a (non-`void`) type R, assume the lambda's parameter types are the same as the function type's parameter types. Then:


        - If R is a proper type, and if the lambda body or some result expression in the lambda body is not compatible in an assignment context with R, then *false*.

        - Otherwise, if R is not a proper type, then where the lambda body has the form *Expression*, the constraint ‹*Expression* → R›; or where the lambda body is a block with result expressions `e`<sub>`1`</sub>, ..., `e`<sub>`m`</sub>, for all *i* (1 ≤ *i* ≤ *m*), ‹`e`<sub>`i`</sub> → R›.

        

      

    

  


The key piece of information to derive from a compatibility constraint involving a lambda expression is the set of bounds on inference variables appearing in the target function type's return type. This is crucial, because functional interfaces are often generic, and many methods operating on these types are generic, too.

In the simplest case, a lambda expression may simply provide a lower bound for an inference variable:

``` screen

<T> List<T> makeThree(Factory<T> factory) { ... }
String s = makeThree(() -> "abc").get(2);
```

In more complex cases, a result expression may be a poly expression - perhaps even another lambda expression - and so the inference variable might be passed through multiple constraint formulas with different target types before a bound is produced.

Most of the work described in this section precedes assertions about the result expressions; its purpose is to derive the lambda expression's function type, and to check for expressions that are clearly disqualified from compatibility.

We do *not* attempt to produce bounds on inference variables that appear in the target function type's `throws` clause. This is because exception containment is not part of compatibility ([§15.27.3](ch15-expressions.md#jls-15.27.3)) - in particular, it must not influence method applicability ([§18.5.1](ch18-type-inference.md#jls-18.5.1)). However, we *do* get bounds on these variables later, because invocation type inference ([§18.5.2.2](ch18-type-inference.md#jls-18.5.2.2)) produces exception containment constraint formulas ([§18.2.5](ch18-type-inference.md#jls-18.2.5)).

Note that if the target type is an inference variable, or if the target type's parameter types contain inference variables, we produce *false*. During invocation type inference ([§18.5.2.2](ch18-type-inference.md#jls-18.5.2.2)), extra substitutions are performed in order to instantiate these inference variables, thus avoiding this scenario. (In other words, reduction will, in practice, never be "invoked" with a target type of one of these forms.)

Finally, note that the result expressions of a lambda expression are required by [§15.27.3](ch15-expressions.md#jls-15.27.3) to be compatible in an assignment context with the target type's return type, R. If R is a proper type, such as `Byte` derived from `Function``<``α``,``Byte``>`, then assignability is easy enough to test, and reduction does so above. If R is not a proper type, such as α derived from `Function``<``String,``α``>`, then we make the simplifying assumption above that loose invocation compatibility will be sufficient. The difference between assignment compatibility and loose invocation compatibility is that only assignment allows narrowing of constant expressions, such as `Byte b = 100;`. Consequently, our simplifying assumption is not completeness-preserving: given target return type α and an integer literal result expression `100`, it is conceivable that α could be instantiated to `Byte`, but reduction will not in fact produce such a bound.

A constraint formula of the form ‹*MethodReference* → T›, where T mentions at least one inference variable, is reduced as follows:


- If T is not a functional interface type, or if T is a functional interface type that does not have a function type ([§9.9](ch09-interfaces.md#jls-9.9)), the constraint reduces to *false*.

- Otherwise, if there does not exist a potentially applicable method for the method reference when targeting T, the constraint reduces to *false*.

- Otherwise, if the method reference is exact ([§15.13.1](ch15-expressions.md#jls-15.13.1)), then let P<sub>1</sub>, ..., P<sub>n</sub> be the parameter types of the function type of T, and let F<sub>1</sub>, ..., F<sub>k</sub> be the parameter types of the potentially applicable method. The constraint reduces to a new set of constraints, as follows:


  - In the special case where *n* = *k*+1, the parameter of type P<sub>1</sub> is to act as the target reference of the invocation. The method reference expression necessarily has the form *ReferenceType `::` \[TypeArguments\] Identifier*. The constraint reduces to ‹P<sub>1</sub> `<:` *ReferenceType*› and, for all *i* (2 ≤ *i* ≤ *n*), ‹P<sub>i</sub> → F<sub>i-1</sub>›.

    In all other cases, *n* = *k*, and the constraint reduces to, for all *i* (1 ≤ *i* ≤ *n*), ‹P<sub>i</sub> → F<sub>i</sub>›.

  - If the function type's result is not `void`, let R be its return type. Then, if the result of the potentially applicable compile-time declaration is `void`, the constraint reduces to *false*. Otherwise, the constraint reduces to ‹R' → R›, where R' is the result of applying capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) to the return type of the potentially applicable compile-time declaration.

  

- Otherwise, the method reference is inexact, and:


  - If one or more of the function type's parameter types is not a proper type, the constraint reduces to *false*.

    This condition never arises in practice, due to the handling of inexact method references in [§18.5.1](ch18-type-inference.md#jls-18.5.1) and the substitution applied to the target type in [§18.5.2.2](ch18-type-inference.md#jls-18.5.2.2).

  - Otherwise, a search for a compile-time declaration is performed, as specified in [§15.13.1](ch15-expressions.md#jls-15.13.1). If there is no compile-time declaration for the method reference, the constraint reduces to *false*. Otherwise, there is a compile-time declaration, and: (let R be the result of the function type)


    - If R is `void`, the constraint reduces to *true*.

    - Otherwise, if the method reference expression elides *TypeArguments*, and the compile-time declaration is a generic method, and the return type of the compile-time declaration mentions at least one of the method's type parameters, then:


      - If R mentions one of the type parameters of the function type, the constraint reduces to *false*.

        In this case, a constraint in terms of R might lead an inference variable to be bound by an out-of-scope type variable. Since instantiating an inference variable with an out-of-scope type variable is nonsensical, we prefer to avoid the situation by giving up immediately whenever the possibility arises. This simplification is not completeness-preserving.

      - If R does not mention one of the type parameters of the function type, then the constraint reduces to the bound set B<sub>3</sub> which would be used to determine the method reference's compatibility when targeting the return type of the function type, as defined in [§18.5.2.1](ch18-type-inference.md#jls-18.5.2.1). B<sub>3</sub> may contain new inference variables, as well as dependencies between these new variables and the inference variables in T.

        The strategy used to determine a return type for a generic referenced method follows the pattern used earlier in this section for generic method invocations. This may involve "lifting" bounds into the outer context and inferring dependencies between the two sets of inference variables.

      

    - Otherwise, let R' be the result of applying capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) to the return type of the invocation type ([§15.12.2.6](ch15-expressions.md#jls-15.12.2.6)) of the compile-time declaration. If R' is `void`, the constraint reduces to *false*; otherwise, the constraint reduces to ‹R' → R›.

    

  


### 18.2.2. Type Compatibility Constraints


A constraint formula of the form ‹S → T› is reduced as follows:


- If S and T are proper types, the constraint reduces to *true* if S is compatible in a loose invocation context with T ([§5.3](ch05-conversions-contexts.md#jls-5.3)), and *false* otherwise.

- Otherwise, if S is a primitive type, let S' be the result of applying boxing conversion ([§5.1.7](ch05-conversions-contexts.md#jls-5.1.7)) to S. Then the constraint reduces to ‹S' → T›.

- Otherwise, if T is a primitive type, let T' be the result of applying boxing conversion ([§5.1.7](ch05-conversions-contexts.md#jls-5.1.7)) to T. Then the constraint reduces to ‹S = T'›.

- Otherwise, if T is a parameterized type of the form G`<`T<sub>1</sub>, ..., T<sub>n</sub>`>`, and there exists no type of the form G`<`...`>` that is a supertype of S, but the raw type G is a supertype of S, then the constraint reduces to *true*.

- Otherwise, if T is an array type of the form G`<`T<sub>1</sub>, ..., T<sub>n</sub>`>``[]`<sup>k</sup>, and there exists no type of the form G`<`...`>``[]`<sup>k</sup> that is a supertype of S, but the raw type G`[]`<sup>k</sup> is a supertype of S, then the constraint reduces to *true*. (The notation `[]`<sup>k</sup> indicates an array type of *k* dimensions.)

- Otherwise, the constraint reduces to ‹S `<:` T›.


The fourth and fifth cases are implicit uses of unchecked conversion ([§5.1.9](ch05-conversions-contexts.md#jls-5.1.9)). These, along with any use of unchecked conversion in the first case, may result in compile-time unchecked warnings, and may influence a method's invocation type ([§15.12.2.6](ch15-expressions.md#jls-15.12.2.6)).

Boxing T to T' is not completeness-preserving; for example, if T were `long`, S might be instantiated to `Integer`, which is not a subtype of `Long` but could be unboxed and then widened to `long`. We avoid this problem in most cases by giving special treatment to inference-variable return types that we know are already constrained to be certain boxed primitive types; see [§18.5.2.1](ch18-type-inference.md#jls-18.5.2.1).

Similarly, the treatment of unchecked conversion sacrifices completeness in cases in which T is not a parameterized type (for example, if T is an inference variable). It is not usually clear in such situations whether the unchecked conversion is necessary or not. Since unchecked conversions introduce unchecked warnings, inference prefers to avoid them unless it is clearly necessary.


### 18.2.3. Subtyping Constraints


A constraint formula of the form ‹S `<:` T› is reduced as follows:


- If S and T are proper types, the constraint reduces to *true* if S is a subtype of T ([§4.10](ch04-types-values-variables.md#jls-4.10)), and *false* otherwise.

- Otherwise, if S is the null type, the constraint reduces to *true*.

- Otherwise, if T is the null type, the constraint reduces to *false*.

- Otherwise, if S is an inference variable, α, the constraint reduces to the bound α `<:` T.

- Otherwise, if T is an inference variable, α, the constraint reduces to the bound S `<:` α.

- Otherwise, the constraint is reduced according to the form of T:


  - If T is a parameterized class or interface type, or an inner class type of a parameterized class or interface type (directly or indirectly), let A<sub>1</sub>, ..., A<sub>n</sub> be the type arguments of T. Among the supertypes of S, a corresponding class or interface type is identified, with type arguments B<sub>1</sub>, ..., B<sub>n</sub>. If no such type exists, the constraint reduces to *false*. Otherwise, the constraint reduces to the following new constraints: for all *i* (1 ≤ *i* ≤ *n*), ‹B<sub>i</sub> `<=` A<sub>i</sub>›.

  - If T is any other class or interface type, then the constraint reduces to *true* if T is among the supertypes of S, and *false* otherwise.

  - If T is an array type, T'`[]`, then among the supertypes of S that are array types, a most specific type is identified, S'`[]` (this may be S itself). If no such array type exists, the constraint reduces to *false*. Otherwise:


    - If neither S' nor T' is a primitive type, the constraint reduces to ‹S' `<:` T'›.

    - Otherwise, the constraint reduces to *true* if S' and T' are the same primitive type, and *false* otherwise.

    

  - If T is a type variable, there are three cases:


    - If S is an intersection type of which T is an element, the constraint reduces to *true*.

    - Otherwise, if T has a lower bound, B, the constraint reduces to ‹S `<:` B›.

    - Otherwise, the constraint reduces to *false*.

    

  - If T is an intersection type, I<sub>1</sub> `&` ... `&` I<sub>n</sub>, the constraint reduces to the following new constraints: for all *i* (1 ≤ *i* ≤ *n*), ‹S `<:` I<sub>i</sub>›.

  


A constraint formula of the form ‹S `<=` T›, where S and T are type arguments ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)), is reduced as follows:


- If T is a type:


  - If S is a type, the constraint reduces to ‹S = T›.

  - If S is a wildcard, the constraint reduces to *false*.

  

- If T is a wildcard of the form `?`, the constraint reduces to *true*.

- If T is a wildcard of the form `?` `extends` T':


  - If S is a type, the constraint reduces to ‹S `<:` T'›.

  - If S is a wildcard of the form `?`, the constraint reduces to ‹`Object` `<:` T'›.

  - If S is a wildcard of the form `?` `extends` S', the constraint reduces to ‹S' `<:` T'›.

  - If S is a wildcard of the form `?` `super` S', the constraint reduces to ‹`Object` = T'›.

  

- If T is a wildcard of the form `?` `super` T':


  - If S is a type, the constraint reduces to ‹T' `<:` S›.

  - If S is a wildcard of the form `?` `super` S', the constraint reduces to ‹T' `<:` S'›.

  - Otherwise, the constraint reduces to *false*.

  


### 18.2.4. Type Equality Constraints


A constraint formula of the form ‹S = T›, where S and T are types, is reduced as follows:


- If S and T are proper types, the constraint reduces to *true* if S is the same as T ([§4.3.4](ch04-types-values-variables.md#jls-4.3.4)), and *false* otherwise.

- Otherwise, if S or T is the null type, the constraint reduces to *false*.

- Otherwise, if S is an inference variable, α, and T is not a primitive type, the constraint reduces to the bound α = T.

- Otherwise, if T is an inference variable, α, and S is not a primitive type, the constraint reduces to the bound S = α.

- Otherwise, if S and T are class or interface types with the same erasure, where S has type arguments B<sub>1</sub>, ..., B<sub>n</sub> and T has type arguments A<sub>1</sub>, ..., A<sub>n</sub>, the constraint reduces to the following new constraints: for all *i* (1 ≤ *i* ≤ *n*), ‹B<sub>i</sub> = A<sub>i</sub>›.

- Otherwise, if S and T are array types, S'`[]` and T'`[]`, the constraint reduces to ‹S' = T'›.

- Otherwise, if S and T are intersection types, a correspondence between the elements of S and the elements of T is established. An element of S, S<sub>i</sub>, corresponds to an element of T, T<sub>j</sub>, if S<sub>i</sub> and T<sub>j</sub> are either the same type, or both parameterizations of the same generic class or interface, or both array types.

  If each element of S corresponds to exactly one element of T, and vice versa, then the constraint reduces to the following new constraints: for each element S<sub>i</sub> of S and the corresponding element T<sub>j</sub> of T, ‹S<sub>i</sub> = T<sub>j</sub>›. If not, the constraint reduces to *false*.

  This rule does not accommodate inference variables appearing directly as elements of an intersection type (rather than nested in a parameterized type). Due to the restrictions on type parameter declarations ([§4.4](ch04-types-values-variables.md#jls-4.4)), such intersection types do not arise in practice.

- Otherwise, the constraint reduces to *false*.


A constraint formula of the form ‹S = T›, where S and T are type arguments ([§4.5.1](ch04-types-values-variables.md#jls-4.5.1)), is reduced as follows:


- If S and T are types, the constraint is reduced as described above.

- If S has the form `?` and T has the form `?`, the constraint reduces to *true*.

- If S has the form `?` and T has the form `?` `extends` T', the constraint reduces to ‹`Object` = T'›.

- If S has the form `?` `extends` S' and T has the form `?`, the constraint reduces to ‹S' = `Object`›.

- If S has the form `?` `extends` S' and T has the form `?` `extends` T', the constraint reduces to ‹S' = T'›.

- If S has the form `?` `super` S' and T has the form `?` `super` T', the constraint reduces to ‹S' = T'›.

- Otherwise, the constraint reduces to *false*.


### 18.2.5. Checked Exception Constraints


A constraint formula of the form ‹*LambdaExpression* →<sub>*throws*</sub> T› is reduced as follows:


- If T is not a functional interface type ([§9.8](ch09-interfaces.md#jls-9.8)), the constraint reduces to *false*.

- Otherwise, let the target function type for the lambda expression be determined as specified in [§15.27.3](ch15-expressions.md#jls-15.27.3). If no valid function type can be found, the constraint reduces to *false*.

- Otherwise, if the lambda expression is implicitly typed, and one or more of the function type's parameter types is not a proper type, the constraint reduces to *false*.

  This condition never arises in practice, due to the substitution applied to the target type in [§18.5.2.2](ch18-type-inference.md#jls-18.5.2.2).

- Otherwise, if the function type's return type is neither `void` nor a proper type, the constraint reduces to *false*.

  This condition never arises in practice, due to the substitution applied to the target type in [§18.5.2.2](ch18-type-inference.md#jls-18.5.2.2).

- Otherwise, let E<sub>1</sub>, ..., E<sub>n</sub> be the types in the function type's `throws` clause that are *not* proper types. If the lambda expression is implicitly typed, let its parameter types be the function type's parameter types. If the lambda body is a poly expression or a block containing a poly result expression, let the targeted return type be the function type's return type. Let X<sub>1</sub>, ..., X<sub>m</sub> be the checked exception types that the lambda body can throw ([§11.2](ch11-exceptions.md#jls-11.2)). Then there are two cases:


  - If *n* = `0` (the function type's `throws` clause consists only of proper types), then if there exists some *i* (1 ≤ *i* ≤ *m*) such that X<sub>i</sub> is not a subtype of any proper type in the `throws` clause, the constraint reduces to *false*; otherwise, the constraint reduces to *true*.

  - If *n* \> `0`, the constraint reduces to a set of subtyping constraints: for all *i* (1 ≤ *i* ≤ *m*), if X<sub>i</sub> is not a subtype of any proper type in the `throws` clause, then the constraints include, for all *j* (1 ≤ *j* ≤ *n*), ‹X<sub>i</sub> `<:` E<sub>j</sub>›. In addition, for all *j* (1 ≤ *j* ≤ *n*), the constraint reduces to the bound `throws` E<sub>j</sub>.

  


A constraint formula of the form ‹*MethodReference* →<sub>*throws*</sub> T› is reduced as follows:


- If T is not a functional interface type, or if T is a functional interface type but does not have a function type ([§9.9](ch09-interfaces.md#jls-9.9)), the constraint reduces to *false*.

- Otherwise, let the target function type for the method reference expression be the function type of T. If the method reference is inexact ([§15.13.1](ch15-expressions.md#jls-15.13.1)) and one or more of the function type's parameter types is not a proper type, the constraint reduces to *false*.

- Otherwise, if the method reference is inexact and the function type's result is neither `void` nor a proper type, the constraint reduces to *false*.

- Otherwise, let E<sub>1</sub>, ..., E<sub>n</sub> be the types in the function type's `throws` clause that are *not* proper types. Let X<sub>1</sub>, ..., X<sub>m</sub> be the checked exceptions in the `throws` clause of the invocation type of the method reference's compile-time declaration ([§15.13.2](ch15-expressions.md#jls-15.13.2)) (as derived from the function type's parameter types and return type). Then there are two cases:


  - If *n* = `0` (the function type's `throws` clause consists only of proper types), then if there exists some *i* (1 ≤ *i* ≤ *m*) such that X<sub>i</sub> is not a subtype of any proper type in the `throws` clause, the constraint reduces to *false*; otherwise, the constraint reduces to *true*.

  - If *n* \> `0`, the constraint reduces to a set of subtyping constraints: for all *i* (1 ≤ *i* ≤ *m*), if X<sub>i</sub> is not a subtype of any proper type in the `throws` clause, then the constraints include, for all *j* (1 ≤ *j* ≤ *n*), ‹X<sub>i</sub> `<:` E<sub>j</sub>›. In addition, for all *j* (1 ≤ *j* ≤ *n*), the constraint reduces to the bound `throws` E<sub>j</sub>.

  


Constraints on checked exceptions are handled separately from constraints on return types, because return type compatibility influences applicability of methods ([§18.5.1](ch18-type-inference.md#jls-18.5.1)), while exceptions only influence the invocation type after overload resolution is complete ([§18.5.2](ch18-type-inference.md#jls-18.5.2)). This could be simplified by including exception compatibility in the definition of lambda expression compatibility ([§15.27.3](ch15-expressions.md#jls-15.27.3)), but this would lead to possibly surprising cases in which exceptions that can be thrown by an explicitly typed lambda body change overload resolution.

The exceptions thrown by a lambda body cannot be determined until (i) the parameter types of the lambda are known, and (ii) the target type of result expressions in the body is known. (The second requirement is to account for generic method invocations in which, for example, the same type parameter appears in the return type and the `throws` clause.) Hence, we require both of these, as derived from the target type T, to be proper types.

One consequence is that lambda expressions returned from *other* lambda expressions cannot generate constraints from their thrown exceptions. These constraints can only be generated from top-level lambda expressions.

Note that the handling of the case in which more than one inference variable appears in a function type's `throws` clause is not completeness-preserving. Either variable may, on its own, satisfy the constraint that each checked exception be declared, but we cannot be sure which one is intended. So, for predictability, we constrain them both.


## 18.3. Incorporation


As bound sets are generated and grown during inference, it is possible that new bounds can be inferred based on the assertions of the original bounds. The process of *incorporation* identifies these new bounds and adds them to the bound set.

Incorporation can happen in two scenarios. One scenario is that the bound set contains complementary pairs of bounds; this implies new constraint formulas, as specified in [§18.3.1](ch18-type-inference.md#jls-18.3.1). The other scenario is that the bound set contains a bound involving capture conversion; this implies new bounds and may imply new constraint formulas, as specified in [§18.3.2](ch18-type-inference.md#jls-18.3.2). In both scenarios, any new constraint formulas are reduced, and any new bounds are added to the bound set. This may trigger further incorporation; ultimately, the set will reach a fixed point and no further bounds can be inferred.

If incorporation of a bound set has reached a fixed point, and the set does not contain the bound *false*, then the bound set has the following properties:


- For each combination of a proper lower bound `L` and a proper upper bound U of an inference variable, `L` `<:` U.

- If every inference variable mentioned by a bound has an instantiation, the bound is satisfied by the corresponding substitution.

- Given a dependency α = β, every bound of α matches a bound of β, and vice versa.

- Given a dependency α `<:` β, every lower bound of α is a lower bound of β, and every upper bound of β is an upper bound of α.


The assertion that incorporation reaches a fixed point oversimplifies the matter slightly. Building on the work of Kennedy and Pierce, *[On Decidability of Nominal Subtyping with Variance](http://research.microsoft.com/apps/pubs/default.aspx?id=64041)*, this property can be proven by making the argument that the set of types that may appear in the bound set is finite. The argument relies on two assumptions:


- New capture variables are not generated when reducing subtyping constraints ([§18.2.3](ch18-type-inference.md#jls-18.2.3)).

- Expansive inheritance paths are not pursued.


This specification does not currently guarantee these properties (it is imprecise about the handling of wildcards when reducing subtyping constraints, and does not detect expansive inheritance paths), but may do so in a future version. (This is not a new problem: the Java subtyping algorithm is also at risk of non-termination.)


### 18.3.1. Complementary Pairs of Bounds


(In this section, S and T are inference variables or types, and U is a proper type. For conciseness, a bound of the form α = T may also match a bound of the form T = α.)

When a bound set contains a pair of bounds that match one of the following rules, a new constraint formula is implied:


- α = S and α = T imply ‹S = T›

- α = S and α `<:` T imply ‹S `<:` T›

- α = S and T `<:` α imply ‹T `<:` S›

- S `<:` α and α `<:` T imply ‹S `<:` T›

- α = U and S = T imply ‹S`[`α:=U`]` = T`[`α:=U`]`›

- α = U and S `<:` T imply ‹S`[`α:=U`]` `<:` T`[`α:=U`]`›


When a bound set contains a pair of bounds α `<:` S and α `<:` T, and there exists a supertype of S of the form G`<`S<sub>1</sub>, ..., S<sub>n</sub>`>` and a supertype of T of the form G`<`T<sub>1</sub>, ..., T<sub>n</sub>`>` (for some generic class or interface, G), then for all *i* (1 ≤ *i* ≤ *n*), if S<sub>i</sub> and T<sub>i</sub> are types (not wildcards), the constraint formula ‹S<sub>i</sub> = T<sub>i</sub>› is implied.


### 18.3.2. Bounds Involving Capture Conversion


When a bound set contains a bound of the form G`<`α<sub>1</sub>, ..., α<sub>n</sub>`>` = capture(G`<`A<sub>1</sub>, ..., A<sub>n</sub>`>`), new bounds are implied and new constraint formulas may be implied, as follows.

Let P<sub>1</sub>, ..., P<sub>n</sub> represent the type parameters of G and let B<sub>1</sub>, ..., B<sub>n</sub> represent the bounds of these type parameters. Let θ represent the substitution `[`P<sub>1</sub>:=α<sub>1</sub>, ..., P<sub>n</sub>:=α<sub>n</sub>`]`. Let R be a type that is *not* an inference variable (but is not necessarily a proper type).

A set of bounds on α<sub>1</sub>, ..., α<sub>n</sub> is implied, generated from the declared bounds of P<sub>1</sub>, ..., P<sub>n</sub> as specified in [§18.1.3](ch18-type-inference.md#jls-18.1.3).

In addition, for all *i* (1 ≤ *i* ≤ *n*):


- If A<sub>i</sub> is not a wildcard, then the bound α<sub>i</sub> = A<sub>i</sub> is implied.

- If A<sub>i</sub> is a wildcard of the form `?`:


  - α<sub>i</sub> = R implies the bound *false*

  - α<sub>i</sub> `<:` R implies the constraint formula ‹B<sub>i</sub> θ `<:` R›

  - R `<:` α<sub>i</sub> implies the bound *false*

  

- If A<sub>i</sub> is a wildcard of the form `?` `extends` T:


  - α<sub>i</sub> = R implies the bound *false*

  - If B<sub>i</sub> is `Object`, then α<sub>i</sub> `<:` R implies the constraint formula ‹T `<:` R›

  - If T is `Object`, then α<sub>i</sub> `<:` R implies the constraint formula ‹B<sub>i</sub> θ `<:` R›

  - R `<:` α<sub>i</sub> implies the bound *false*

  

- If A<sub>i</sub> is a wildcard of the form `?` `super` T:


  - α<sub>i</sub> = R implies the bound *false*

  - α<sub>i</sub> `<:` R implies the constraint formula ‹B<sub>i</sub> θ `<:` R›

  - R `<:` α<sub>i</sub> implies the constraint formula ‹R `<:` T›

  


## 18.4. Resolution


Given a bound set that does not contain the bound *false*, a subset of the inference variables mentioned by the bound set may be *resolved*. This means that a satisfactory instantiation may be added to the set for each inference variable, until all the requested variables have instantiations.

Dependencies in the bound set may require that the variables be resolved in a particular order, or that additional variables be resolved. Dependencies are specified as follows:


- Given a bound of one of the following forms, where T is either an inference variable β or a type that mentions β:


  - α = T

  - α `<:` T

  - T = α

  - T `<:` α

  

  If α appears on the left-hand side of another bound of the form G`<`..., α, ...`>` = capture(G`<`...`>`), then β depends on the resolution of α. Otherwise, α depends on the resolution of β.

- An inference variable α appearing on the left-hand side of a bound of the form G`<`..., α, ...`>` = capture(G`<`...`>`) depends on the resolution of every other inference variable mentioned in this bound (on both sides of the = sign).

- An inference variable α depends on the resolution of an inference variable β if there exists an inference variable γ such that α depends on the resolution of γ and γ depends on the resolution of β.

- An inference variable α depends on the resolution of itself.


Given a set of inference variables to resolve, let V be the union of this set and all variables upon which the resolution of at least one variable in this set depends.

If every variable in V has an instantiation, then resolution succeeds and this procedure terminates.

Otherwise, let { α<sub>1</sub>, ..., α<sub>n</sub> } be a non-empty subset of uninstantiated variables in V such that (i) for all *i* (1 ≤ *i* ≤ *n*), if α<sub>i</sub> depends on the resolution of a variable β, then either β has an instantiation or there is some *j* such that β = α<sub>j</sub>; and (ii) there exists no non-empty proper subset of { α<sub>1</sub>, ..., α<sub>n</sub> } with this property. Resolution proceeds by generating an instantiation for each of α<sub>1</sub>, ..., α<sub>n</sub> based on the bounds in the bound set:


- If the bound set does not contain a bound of the form G`<`..., α<sub>i</sub>, ...`>` = capture(G`<`...`>`) for all *i* (1 ≤ *i* ≤ *n*), then a candidate instantiation T<sub>i</sub> is defined for each α<sub>i</sub>:


  - If α<sub>i</sub> has one or more *proper* lower bounds, `L`<sub>`1`</sub>, ..., `L`<sub>`k`</sub>, then T<sub>i</sub> = lub(`L`<sub>`1`</sub>, ..., `L`<sub>`k`</sub>) ([§4.10.4](ch04-types-values-variables.md#jls-4.10.4)).

  - Otherwise, if the bound set contains `throws` α<sub>i</sub>, and each proper upper bound of α<sub>i</sub> is a supertype of `RuntimeException`, then T<sub>i</sub> = `RuntimeException`.

  - Otherwise, where α<sub>i</sub> has *proper* upper bounds U<sub>1</sub>, ..., U<sub>k</sub>, T<sub>i</sub> = glb(U<sub>1</sub>, ..., U<sub>k</sub>) ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)).

  

  The bounds α<sub>1</sub> = T<sub>1</sub>, ..., α<sub>n</sub> = T<sub>n</sub> are incorporated with the current bound set.

  If the result does not contain the bound *false*, then the result becomes the new bound set, and resolution proceeds by selecting a new set of variables to instantiate (if necessary), as described above.

  Otherwise, the result contains the bound *false*, so a second attempt is made to instantiate { α<sub>1</sub>, ..., α<sub>n</sub> } by performing the step below.

- If the bound set contains a bound of the form G`<`..., α<sub>i</sub>, ...`>` = capture(G`<`...`>`) for some *i* (1 ≤ *i* ≤ *n*), or;

  If the bound set produced in the step above contains the bound *false*;

  then let Y<sub>1</sub>, ..., Y<sub>n</sub> be fresh type variables whose bounds are as follows:


  - For all *i* (1 ≤ *i* ≤ *n*), if α<sub>i</sub> has one or more *proper* lower bounds `L`<sub>`1`</sub>, ..., `L`<sub>`k`</sub>, then let the lower bound of Y<sub>i</sub> be lub(`L`<sub>`1`</sub>, ..., `L`<sub>`k`</sub>); if not, then Y<sub>i</sub> has no lower bound.

  - For all *i* (1 ≤ *i* ≤ *n*), where α<sub>i</sub> has upper bounds U<sub>1</sub>, ..., U<sub>k</sub>, let the upper bound of Y<sub>i</sub> be glb(U<sub>1</sub> θ, ..., U<sub>k</sub> θ), where θ is the substitution `[`α<sub>1</sub>:=Y<sub>1</sub>, ..., α<sub>n</sub>:=Y<sub>n</sub>`]`.

  

  If the type variables Y<sub>1</sub>, ..., Y<sub>n</sub> do not have well-formed bounds (that is, a lower bound is not a subtype of an upper bound, or an intersection type is inconsistent), then resolution fails.

  Otherwise, for all *i* (1 ≤ *i* ≤ *n*), all bounds of the form G`<`..., α<sub>i</sub>, ...`>` = capture(G`<`...`>`) are removed from the current bound set, and the bounds α<sub>1</sub> = Y<sub>1</sub>, ..., α<sub>n</sub> = Y<sub>n</sub> are incorporated.

  If the result does not contain the bound *false*, then the result becomes the new bound set, and resolution proceeds by selecting a new set of variables to instantiate (if necessary), as described above.

  Otherwise, the result contains the bound *false*, and resolution fails.


The first method of instantiating an inference variable derives the instantiation from that variable's bounds. Sometimes, however, complex dependencies mean that the result is not within the variable's bounds. In that case, a different method of instantiation is performed, analogous to capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)): fresh type variables are introduced, with bounds derived from the bounds of the inference variables. Note that the lower bounds of these "capture" variables are computed using only proper types: this is important in order to avoid attempts to perform typing computations on uninstantiated type variables.


## 18.5. Uses of Inference


Using the inference processes defined above, the following analyses are performed at compile time.


### 18.5.1. Invocation Applicability Inference


Given a method invocation that provides no explicit type arguments, the process to determine whether a potentially applicable generic method `m` is applicable is as follows:


- Where P<sub>1</sub>, ..., P<sub>p</sub> (*p* ≥ 1) are the type parameters of `m`, let α<sub>1</sub>, ..., α<sub>p</sub> be inference variables, and let θ be the substitution `[`P<sub>1</sub>:=α<sub>1</sub>, ..., P<sub>p</sub>:=α<sub>p</sub>`]`.

- An initial bound set, B<sub>0</sub>, is generated from the declared bounds of P<sub>1</sub>, ..., P<sub>p</sub>, as described in [§18.1.3](ch18-type-inference.md#jls-18.1.3).

- For all *i* (1 ≤ *i* ≤ *p*), if P<sub>i</sub> appears in the `throws` clause of `m`, then the bound `throws` α<sub>i</sub> is implied. These bounds, if any, are incorporated with B<sub>0</sub> to produce a new bound set, B<sub>1</sub>.

- A set of constraint formulas, C, is generated as follows.

  Let F<sub>1</sub>, ..., F<sub>n</sub> be the formal parameter types of `m`, and let `e`<sub>`1`</sub>, ..., `e`<sub>`k`</sub> be the actual argument expressions of the invocation. Then:


  - To test for *applicability by strict invocation*:

    If *k* ≠ *n*, or if there exists an *i* (1 ≤ *i* ≤ *n*) such that `e`<sub>`i`</sub> is pertinent to applicability ([§15.12.2.2](ch15-expressions.md#jls-15.12.2.2)) and either (i) `e`<sub>`i`</sub> is a standalone expression of a primitive type but F<sub>i</sub> is a reference type, or (ii) F<sub>i</sub> is a primitive type but `e`<sub>`i`</sub> is not a standalone expression of a primitive type; then the method is not applicable and there is no need to proceed with inference.

    Otherwise, C includes, for all *i* (1 ≤ *i* ≤ *k*) where `e`<sub>`i`</sub> is pertinent to applicability, ‹`e`<sub>`i`</sub> → F<sub>i</sub> θ›.

  - To test for *applicability by loose invocation*:

    If *k* ≠ *n*, the method is not applicable and there is no need to proceed with inference.

    Otherwise, C includes, for all *i* (1 ≤ *i* ≤ *k*) where `e`<sub>`i`</sub> is pertinent to applicability, ‹`e`<sub>`i`</sub> → F<sub>i</sub> θ›.

  - To test for *applicability by variable arity invocation*:

    Let F'<sub>1</sub>, ..., F'<sub>k</sub> be the first *k* variable arity parameter types of `m` ([§15.12.2.4](ch15-expressions.md#jls-15.12.2.4)). C includes, for all *i* (1 ≤ *i* ≤ *k*) where `e`<sub>`i`</sub> is pertinent to applicability, ‹`e`<sub>`i`</sub> → F'<sub>i</sub> θ›.

  

- C is reduced ([§18.2](ch18-type-inference.md#jls-18.2)) and the resulting bounds are incorporated with B<sub>1</sub> to produce a new bound set, B<sub>2</sub>.

- Finally, the method `m` is applicable if B<sub>2</sub> does not contain the bound *false* and resolution of all the inference variables in B<sub>2</sub> succeeds ([§18.4](ch18-type-inference.md#jls-18.4)).


Consider the following method invocation and assignment:

``` screen
List<Number> ln = Arrays.asList(1, 2.0);
```

A most specific applicable method for the invocation must be identified as described in [§15.12](ch15-expressions.md#jls-15.12). The only potentially applicable method ([§15.12.2.1](ch15-expressions.md#jls-15.12.2.1)) is declared as follows:

``` screen
public static <T> List<T> asList(T... a)
```

Trivially (because of its arity), this method is neither applicable by strict invocation ([§15.12.2.2](ch15-expressions.md#jls-15.12.2.2)) nor applicable by loose invocation ([§15.12.2.3](ch15-expressions.md#jls-15.12.2.3)). But since there are no other candidates, in a third phase the method is checked for applicability by variable arity invocation.

The initial bound set, B, is a trivial upper bound for a single inference variable, α:

{ α `<:` `Object` }

The initial constraint formula set is as follows:

{ ‹`1` → α›, ‹`2.0` → α› }

These are reduced to a new bound set, B<sub>1</sub>:

{ α `<:` `Object`, `Integer` `<:` α, `Double` `<:` α }

Then, to test whether the method is applicable, we attempt to resolve these bounds. We succeed, producing the rather complex instantiation

α = `Number & Comparable<? extends Number & Comparable<?>>`

We have thus demonstrated that the method is applicable; since no other candidates exist, it is the most specific applicable method. Still, the type of the method invocation, and its compatibility with the target type in the assignment, is not determined until further inference can occur, as described in the next section.


### 18.5.2. Invocation Type Inference


Given a method invocation expression that provides no explicit type arguments, and a corresponding most specific applicable generic method `m`, the process to infer the invocation type ([§15.12.2.6](ch15-expressions.md#jls-15.12.2.6)) of the chosen method may require resolving additional constraints, both to assert compatibility with a target type and to assert validity of the method invocation's argument expressions.

It is important to note that multiple "rounds" of inference are involved in finding the type of a method invocation. This is necessary, for example, to allow a target type to influence the type of the invocation without allowing it to influence the choice of an applicable method. The first round ([§18.5.1](ch18-type-inference.md#jls-18.5.1)) produces a bound set and tests that a resolution exists, but does not commit to that resolution. Subsequent rounds reduce additional constraints until a final resolution step determines the "real" type of the expression.


#### 18.5.2.1. Poly Method Invocation Compatibility


If the method invocation expression is a poly expression ([§15.12](ch15-expressions.md#jls-15.12)), its compatibility with a target type T is determined as follows.

If the method invocation expression appears in a strict invocation context and T is a primitive type, the expression is not compatible with T.

Otherwise:


- Let B<sub>2</sub> be the bound set produced by reduction in order to demonstrate that `m` is applicable in [§18.5.1](ch18-type-inference.md#jls-18.5.1).

  (While it was necessary in [§18.5.1](ch18-type-inference.md#jls-18.5.1) to demonstrate that the inference variables in B<sub>2</sub> could be resolved, in order to establish applicability, the instantiations produced by this resolution step are *not* considered part of B<sub>2</sub>.)

- Let B<sub>3</sub> be the bound set derived from B<sub>2</sub> as follows.

  Let R be the return type of `m`, and let θ be the substitution `[`P<sub>1</sub>:=α<sub>1</sub>, ..., P<sub>p</sub>:=α<sub>p</sub>`]` defined in [§18.5.1](ch18-type-inference.md#jls-18.5.1) to replace the type parameters of `m` with inference variables, and let T be the invocation's target type. Then:


  - If unchecked conversion was necessary for the method to be applicable during constraint set reduction in [§18.5.1](ch18-type-inference.md#jls-18.5.1), the constraint formula ‹\|R\| → T› is reduced and incorporated with B<sub>2</sub>.

  - Otherwise, if R θ is a parameterized type, G`<`A<sub>1</sub>, ..., A<sub>n</sub>`>`, and one of A<sub>1</sub>, ..., A<sub>n</sub> is a wildcard, then, for fresh inference variables β<sub>1</sub>, ..., β<sub>n</sub>, the constraint formula ‹G`<`β<sub>1</sub>, ..., β<sub>n</sub>`>` → T› is reduced and incorporated, along with the bound G`<`β<sub>1</sub>, ..., β<sub>n</sub>`>` = capture(G`<`A<sub>1</sub>, ..., A<sub>n</sub>`>`), with B<sub>2</sub>.

  - Otherwise, if R θ is an inference variable α, and one of the following is true:


    - T is a reference type, but is not a wildcard-parameterized type, and either (i) B<sub>2</sub> contains a bound of one of the forms α = S or S `<:` α, where S is a wildcard-parameterized type, or (ii) B<sub>2</sub> contains two bounds of the forms S<sub>1</sub> `<:` α and S<sub>2</sub> `<:` α, where S<sub>1</sub> and S<sub>2</sub> have supertypes that are two different parameterizations of the same generic class or interface.

    - T is a parameterization of a generic class or interface, G, and B<sub>2</sub> contains a bound of one of the forms α = S or S `<:` α, where there exists no type of the form G`<`...`>` that is a supertype of S, but the raw type \|G`<`...`>`\| is a supertype of S.

    - T is a primitive type, and one of the primitive wrapper classes mentioned in [§5.1.7](ch05-conversions-contexts.md#jls-5.1.7) is an instantiation, upper bound, or lower bound for α in B<sub>2</sub>.

    

    then α is resolved in B<sub>2</sub>, and where the capture of the resulting instantiation of α is U, the constraint formula ‹U → T› is reduced and incorporated with B<sub>2</sub>.

  - Otherwise, the constraint formula ‹R θ → T› is reduced and incorporated with B<sub>2</sub>.

  

- The method invocation expression is compatible with T if B<sub>3</sub> does not contain the bound *false* and resolution of all the inference variables in B<sub>3</sub> succeeds ([§18.4](ch18-type-inference.md#jls-18.4)).


Consider the example from the previous section:

``` screen
List<Number> ln = Arrays.asList(1, 2.0);
```

The most specific applicable method was identified as:

``` screen
public static <T> List<T> asList(T... a)
```

In order to complete type-checking of the method invocation, we must determine whether it is compatible with its target type, `List``<``Number``>`.

The bound set used to demonstrate applicability in the previous section, B<sub>2</sub>, was:

{ α `<:` `Object`, `Integer` `<:` α, `Double` `<:` α }

The new constraint formula set is as follows:

{ ‹`List``<``α``>` → `List``<``Number``>`› }

This compatibility constraint produces an equality bound for α, which is included in the new bound set, B<sub>3</sub>:

{ α `<:` `Object`, `Integer` `<:` α, `Double` `<:` α, α = `Number` }

These bounds are trivially resolved:

α = `Number`

Finally, we perform a substitution on the declared return type of `asList` to determine that the method invocation has type `List``<``Number``>`; clearly, this is compatible with the target type.

This inference strategy is different than the Java SE 7 Edition of *The Java Language Specification*, which would have instantiated α based on its lower bounds (before even considering the invocation's target type), as we did in the previous section. This would result in a type error, since the resulting type is not a subtype of `List``<``Number``>`.


Under various special circumstances, based on the bounds appearing in B<sub>2</sub>, we eagerly resolve an inference variable that appears as the return type of the invocation. This is to avoid unfortunate situations in which the usual constraint, ‹R θ → T›, is not completeness-preserving. It is, unfortunately, possible that by eagerly resolving the variable, we are unable to make use of bounds that would be inferred later. It is also possible that, in some cases, bounds that will later be inferred from the invocation arguments (such as implicitly typed lambda expressions) would have caused a different outcome if they had been present in B<sub>2</sub>. Despite these limitations, the strategy allows for reasonable outcomes in typical use cases, and is backwards compatible with the algorithm in the Java SE 7 Edition of *The Java Language Specification*.


#### 18.5.2.2. Additional Argument Constraints


The invocation type for the chosen method is determined after considering additional constraints that may be implied by the argument expressions of the method invocation expression, as follows:


- If the method invocation expression is a poly expression, let B<sub>3</sub> be the bound set generated in [§18.5.2.1](ch18-type-inference.md#jls-18.5.2.1) to demonstrate compatibility with the actual target type of the method invocation.

  If the method invocation expression is not a poly expression, let B<sub>3</sub> be the same as the bound set produced by reduction in order to demonstrate that `m` is applicable in [§18.5.1](ch18-type-inference.md#jls-18.5.1).

  (While it was necessary in [§18.5.1](ch18-type-inference.md#jls-18.5.1) and [§18.5.2.1](ch18-type-inference.md#jls-18.5.2.1) to demonstrate that the inference variables in the bound set could be resolved, the instantiations produced by these resolution steps are *not* considered part of B<sub>3</sub>.)

- A set of constraint formulas, C, is generated as follows.

  Let `e`<sub>`1`</sub>, ..., `e`<sub>`k`</sub> be the actual argument expressions of the method invocation expression.

  If `m` is applicable by strict or loose invocation, let F<sub>1</sub>, ..., F<sub>k</sub> be the formal parameter types of `m`; if `m` is applicable by variable arity invocation, let F<sub>1</sub>, ..., F<sub>k</sub> the first *k* variable arity parameter types of `m` ([§15.12.2.4](ch15-expressions.md#jls-15.12.2.4)).

  Let θ be the substitution `[`P<sub>1</sub>:=α<sub>1</sub>, ..., P<sub>p</sub>:=α<sub>p</sub>`]` defined in [§18.5.1](ch18-type-inference.md#jls-18.5.1) to replace the type parameters of `m` with inference variables.

  Then, for all *i* (1 ≤ *i* ≤ *k*):


  - If `e`<sub>`i`</sub> is not pertinent to applicability, C contains ‹`e`<sub>`i`</sub> → F<sub>i</sub> θ›.

  - Additional constraints may be included, depending on the form of `e`<sub>`i`</sub>:


    - If `e`<sub>`i`</sub> is a *LambdaExpression*, C contains ‹*LambdaExpression* →<sub>*throws*</sub> F<sub>i</sub> θ›, and the lambda body is searched for additional constraints:


      - For a block lambda body, the search is applied recursively to each of its result expressions.

      - For a poly class instance creation expression or a poly method invocation expression , C contains all the constraint formulas that would appear in the set C generated by [§18.5.2](ch18-type-inference.md#jls-18.5.2) when inferring the poly expression's invocation type.

      - For a parenthesized expression, the search is applied recursively to the contained expression.

      - For a conditional expression, the search is applied recursively to the second and third operands.

      - For a lambda expression, the search is applied recursively to the lambda body.

      - For a `switch` expression, the search is applied recursively to each of its result expressions.

      

    - If `e`<sub>`i`</sub> is a *MethodReference*, C contains ‹*MethodReference* →<sub>*throws*</sub> F<sub>i</sub> θ›.

    - If `e`<sub>`i`</sub> is a poly class instance creation expression or a poly method invocation expression, C contains all the constraint formulas that would appear in the set C generated by [§18.5.2](ch18-type-inference.md#jls-18.5.2) when inferring the poly expression's invocation type.

    - If `e`<sub>`i`</sub> is a parenthesized expression, these rules are applied recursively to the contained expression.

    - If `e`<sub>`i`</sub> is a conditional expression, these rules are applied recursively to the second and third operands.

    - If `e`<sub>`i`</sub> is a `switch` expression, these rules are applied recursively to each of its result expressions.

    

  

- While C is not empty, the following process is repeated, starting with the bound set B<sub>3</sub> and accumulating new bounds into a "current" bound set, ultimately producing a new bound set, B<sub>4</sub>:


  1.  A subset of constraints is selected in C, satisfying the property that, for each constraint, no input variable can influence an output variable of any other constraint in C. The terms *input variable* and *output variable* are defined below. An inference variable α *can influence* an inference variable β if α depends on the resolution of β ([§18.4](ch18-type-inference.md#jls-18.4)), or vice versa; or if there exists a third inference variable γ such that α can influence γ and γ can influence β.

      If this subset is empty, then there is a cycle (or cycles) in the graph of dependencies between constraints. In this case, the constraints in C that participate in a dependency cycle (or cycles) and do not depend on any constraints outside of the cycle (or cycles) are considered. A single constraint is selected from these considered constraints, as follows:


      - If any of the considered constraints have the form ‹*Expression* → T›, then the selected constraint is the considered constraint of this form that contains the expression to the left ([§3.5](ch03-lexical-structure.md#jls-3.5)) of the expression of every other considered constraint of this form.

      - If no considered constraint has the form ‹*Expression* → T›, then the selected constraint is the considered constraint that contains the expression to the left of the expression of every other considered constraint.

      

  2.  The selected constraint(s) are removed from C.

  3.  The input variables α<sub>1</sub>, ..., α<sub>m</sub> of all the selected constraint(s) are resolved.

  4.  Where T<sub>1</sub>, ..., T<sub>m</sub> are the instantiations of α<sub>1</sub>, ..., α<sub>m</sub>, the substitution `[`α<sub>1</sub>:=T<sub>1</sub>, ..., α<sub>m</sub>:=T<sub>m</sub>`]` is applied to every constraint.

  5.  The constraint(s) resulting from substitution are reduced and incorporated with the current bound set.

  

- Finally, if B<sub>4</sub> does not contain the bound *false*, the inference variables in B<sub>4</sub> are resolved.

  If resolution succeeds with instantiations T<sub>1</sub>, ..., T<sub>p</sub> for inference variables α<sub>1</sub>, ..., α<sub>p</sub>, let θ' be the substitution `[`P<sub>1</sub>:=T<sub>1</sub>, ..., P<sub>p</sub>:=T<sub>p</sub>`]`. Then:


  - If unchecked conversion was necessary for the method to be applicable during constraint set reduction in [§18.5.1](ch18-type-inference.md#jls-18.5.1), then the parameter types of the invocation type of `m` are obtained by applying θ' to the parameter types of `m`'s type, and the return type and thrown types of the invocation type of `m` are given by the erasure of the return type and thrown types of `m`'s type.

  - If unchecked conversion was not necessary for the method to be applicable, then the invocation type of `m` is obtained by applying θ' to the type of `m`.

  

  If B<sub>4</sub> contains the bound *false*, or if resolution fails, then a compile-time error occurs.


The process of reducing additional argument constraints may require carefully ordering constraint formulas of the forms ‹*Expression* → T›, ‹*LambdaExpression* →<sub>*throws*</sub> T›, and ‹*MethodReference* →<sub>*throws*</sub> T›. To facilitate this ordering, the *input variables* of these constraints are defined as follows:


- For ‹*LambdaExpression* → T›:


  - If T is an inference variable, it is the (only) input variable.

  - If T is a functional interface type, and a function type can be derived from T ([§15.27.3](ch15-expressions.md#jls-15.27.3)), then the input variables include (i) if the lambda expression is implicitly typed, the inference variables mentioned by the function type's parameter types; and (ii) if the function type's return type, R, is not `void`, then for each result expression `e` in the lambda body (or for the body itself if it is an expression), the input variables of ‹`e` → R›.

  - Otherwise, there are no input variables.

  

- For ‹*LambdaExpression* →<sub>*throws*</sub> T›:


  - If T is an inference variable, it is the (only) input variable.

  - If T is a functional interface type, and a function type can be derived, as described in [§15.27.3](ch15-expressions.md#jls-15.27.3), the input variables include (i) if the lambda expression is implicitly typed, the inference variables mentioned by the function type's parameter types; and (ii) the inference variables mentioned by the function type's return type.

  - Otherwise, there are no input variables.

  

- For ‹*MethodReference* → T›:


  - If T is an inference variable, it is the (only) input variable.

  - If T is a functional interface type with a function type, and if the method reference is inexact ([§15.13.1](ch15-expressions.md#jls-15.13.1)), the input variables are the inference variables mentioned by the function type's parameter types.

  - Otherwise, there are no input variables.

  

- For ‹*MethodReference* →<sub>*throws*</sub> T›:


  - If T is an inference variable, it is the (only) input variable.

  - If T is a functional interface type with a function type, and if the method reference is inexact ([§15.13.1](ch15-expressions.md#jls-15.13.1)), the input variables are the inference variables mentioned by the function type's parameter types and the function type's return type.

  - Otherwise, there are no input variables.

  

- For ‹*Expression* → T›, if *Expression* is a parenthesized expression:

  Where the contained expression of *Expression* is *Expression*', the input variables are the input variables of ‹*Expression*' → T›.

- For ‹*ConditionalExpression* → T›:

  Where the conditional expression has the form `e`<sub>`1`</sub> `?` `e`<sub>`2`</sub> `:` `e`<sub>`3`</sub>, the input variables are the input variables of ‹`e`<sub>`2`</sub> → T› and ‹`e`<sub>`3`</sub> → T›.

- For ‹*SwitchExpression* → T›:

  Where the `switch` expression has result expressions `e`<sub>`1`</sub>, ..., `e`<sub>`n`</sub>, the input variables are, for all *i* (1 ≤ *i* ≤ *n*), the input variables of ‹`e`<sub>`i`</sub> → T›.

- For all other constraint formulas, there are no input variables.


The *output variables* of these constraints are all inference variables mentioned by the type on the right-hand side of the constraint, T, that are not input variables.


### 18.5.3. Functional Interface Parameterization Inference


Where a lambda expression with explicit parameter types P<sub>1</sub>, ..., P<sub>n</sub> targets a functional interface type F`<`A<sub>1</sub>, ..., A<sub>m</sub>`>` with at least one wildcard type argument, then a parameterization of F may be derived as the ground target type of the lambda expression as follows.

Let Q<sub>1</sub>, ..., Q<sub>k</sub> be the parameter types of the function type of the type F`<`α<sub>1</sub>, ..., α<sub>m</sub>`>`, where α<sub>1</sub>, ..., α<sub>m</sub> are fresh inference variables.

If *n* ≠ *k*, no valid parameterization exists. Otherwise, a set of constraint formulas is formed with, for all *i* (1 ≤ *i* ≤ *n*), ‹P<sub>i</sub> = Q<sub>i</sub>›. This constraint formula set is reduced to form the bound set B.

If B contains the bound *false*, no valid parameterization exists. Otherwise, a new parameterization of the functional interface type, F`<`A'<sub>1</sub>, ..., A'<sub>m</sub>`>`, is constructed as follows, for 1 ≤ *i* ≤ *m*:


- If B contains an instantiation ([§18.1.3](ch18-type-inference.md#jls-18.1.3)) for α<sub>i</sub>, T, then A'<sub>i</sub> = T.

- Otherwise, A'<sub>i</sub> = A<sub>i</sub>.


If F`<`A'<sub>1</sub>, ..., A'<sub>m</sub>`>` is not a well-formed type (that is, the type arguments are not within their bounds), or if F`<`A'<sub>1</sub>, ..., A'<sub>m</sub>`>` is not a subtype of F`<`A<sub>1</sub>, ..., A<sub>m</sub>`>`, no valid parameterization exists. Otherwise, the inferred parameterization is either F`<`A'<sub>1</sub>, ..., A'<sub>m</sub>`>`, if all the type arguments are types, or the non-wildcard parameterization ([§9.9](ch09-interfaces.md#jls-9.9)) of F`<`A'<sub>1</sub>, ..., A'<sub>m</sub>`>`, if one or more type arguments are still wildcards.

In order to determine the function type of a wildcard-parameterized functional interface, we have to "instantiate" the wildcard type arguments with specific types. The "default" approach is to simply replace the wildcards with their bounds, as described in [§9.8](ch09-interfaces.md#jls-9.8), but this produces spurious errors in cases where a lambda expression has explicit parameter types that do *not* correspond to the wildcard bounds. For example:

``` screen
Predicate<? super Integer> p = (Number n) -> n.equals(23);
```

The lambda expression is a `Predicate<Number>`, which is a subtype of `Predicate<? super Integer>` but not `Predicate<Integer>`. The analysis in this section is used to infer that `Number` is an appropriate choice for the type argument to `Predicate`.

That said, the analysis here, while described in terms of general type inference, is intentionally quite simple. The only constraints are equality constraints, which means that reduction amounts to simple pattern matching. A more powerful strategy might also infer constraints from the body of the lambda expression. But, given possible interactions with inference for surrounding and/or nested generic method invocations, this would introduce a lot of extra complexity.


### 18.5.4. More Specific Method Inference


When testing that one applicable method is *more specific* than another ([§15.12.2.5](ch15-expressions.md#jls-15.12.2.5)), where the second method is generic, it is necessary to test whether some instantiation of the second method's type parameters can be inferred to make the first method more specific than the second.

Let `m`<sub>`1`</sub> be the first method and `m`<sub>`2`</sub> be the second method. Where `m`<sub>`2`</sub> has type parameters P<sub>1</sub>, ..., P<sub>p</sub>, let α<sub>1</sub>, ..., α<sub>p</sub> be inference variables, and let θ be the substitution `[`P<sub>1</sub>:=α<sub>1</sub>, ..., P<sub>p</sub>:=α<sub>p</sub>`]`.

Let `e`<sub>`1`</sub>, ..., `e`<sub>`k`</sub> be the argument expressions of the corresponding invocation. Then:


- If `m`<sub>`1`</sub> and `m`<sub>`2`</sub> are applicable by strict or loose invocation ([§15.12.2.2](ch15-expressions.md#jls-15.12.2.2), [§15.12.2.3](ch15-expressions.md#jls-15.12.2.3)), then let S<sub>1</sub>, ..., S<sub>k</sub> be the formal parameter types of `m`<sub>`1`</sub>, and let T<sub>1</sub>, ..., T<sub>k</sub> be the result of θ applied to the formal parameter types of `m`<sub>`2`</sub>.

- If `m`<sub>`1`</sub> and `m`<sub>`2`</sub> are applicable by variable arity invocation ([§15.12.2.4](ch15-expressions.md#jls-15.12.2.4)), then let S<sub>1</sub>, ..., S<sub>k</sub> be the first *k* variable arity parameter types of `m`<sub>`1`</sub>, and let T<sub>1</sub>, ..., T<sub>k</sub> be the result of θ applied to the first *k* variable arity parameter types of `m`<sub>`2`</sub>.


Note that no substitution is applied to S<sub>1</sub>, ..., S<sub>k</sub>; even if `m`<sub>`1`</sub> is generic, the type parameters of `m`<sub>`1`</sub> are treated as type variables, not inference variables.

The process to determine if `m`<sub>`1`</sub> is more specific than `m`<sub>`2`</sub> is as follows:


- First, an initial bound set, B, is generated from the declared bounds of P<sub>1</sub>, ..., P<sub>p</sub>, as specified in [§18.1.3](ch18-type-inference.md#jls-18.1.3).

- Second, for all *i* (1 ≤ *i* ≤ *k*), a set of constraint formulas or bounds is generated.

  If T<sub>i</sub> is a proper type, the result is *true* if S<sub>i</sub> is more specific than T<sub>i</sub> for `e`<sub>`i`</sub> ([§15.12.2.5](ch15-expressions.md#jls-15.12.2.5)), and *false* otherwise. (Note that S<sub>i</sub> is always a proper type.)

  Otherwise, if S<sub>i</sub> and T<sub>i</sub> are not both functional interface types, the constraint formula ‹S<sub>i</sub> `<:` T<sub>i</sub>› is generated.

  Otherwise, if the interface of S<sub>i</sub> is a superinterface or a subinterface of the interface of T<sub>i</sub> (or, where S<sub>i</sub> or T<sub>i</sub> is an intersection type, some interface of S<sub>i</sub> is a superinterface or a subinterface of some interface of T<sub>i</sub>), the constraint formula ‹S<sub>i</sub> `<:` T<sub>i</sub>› is generated.

  Otherwise, let MT<sub>S</sub> be the function type of the capture of S<sub>i</sub>, let MT<sub>S</sub>' be the function type of S<sub>i</sub> (without capture), and let MT<sub>T</sub> be the function type of T<sub>i</sub>. If MT<sub>S</sub> and MT<sub>T</sub> have a different number of formal parameters or type parameters, or if MT<sub>S</sub> and MT<sub>S</sub>' do not have the same type parameters ([§8.4.4](ch08-classes.md#jls-8.4.4)), the result is *false*. Otherwise, the following constraint formulas or bounds are generated from the type parameters, formal parameter types, and return types of MT<sub>S</sub> and MT<sub>T</sub>:


  - Let A<sub>1</sub>, ..., A<sub>n</sub> be the type parameters of MT<sub>S</sub>, and let B<sub>1</sub>, ..., B<sub>n</sub> be the type parameters of MT<sub>T</sub>.

    Let θ' be the substitution `[`B<sub>1</sub>:=A<sub>1</sub>, ..., B<sub>n</sub>:=A<sub>n</sub>`]`. Then, for all *j* (1 ≤ *j* ≤ *n*):


    - If the bound of A<sub>j</sub> mentions one of A<sub>1</sub>, ..., A<sub>n</sub>, and the bound of B<sub>j</sub> is a not proper type, *false*.

    - Otherwise, where X is the bound of A<sub>j</sub> and Y is the bound of B<sub>j</sub>, ‹X = Y θ'›.

    

    If the bound A<sub>j</sub> mentions one of A<sub>1</sub>, ..., A<sub>n</sub>, and the bound of B<sub>j</sub> is not a proper type, then producing an equality constraint would raise the possibility of an inference variable being bounded by an out-of-scope type variable. Since instantiating an inference variable with an out-of-scope type variable is nonsensical, we prefer to avoid the situation by giving up immediately whenever the possibility arises. This simplification is not completeness-preserving. (The same comment applies to the treatment of formal parameter types and return types below.)

  - Let U<sub>1</sub>, ..., U<sub>k</sub> be the formal parameter types of MT<sub>S</sub>, and let V<sub>1</sub>, ..., V<sub>k</sub> be the formal parameter types of MT<sub>T</sub>. Then, for all *j* (1 ≤ *j* ≤ *k*):


    - If U<sub>j</sub> mentions one of A<sub>1</sub>, ..., A<sub>n</sub>, and V<sub>j</sub> is not a proper type, *false*.

    - Otherwise, ‹V<sub>j</sub> θ' `<:` U<sub>j</sub>›, and, where U<sub>1</sub>', ..., U<sub>k</sub>' are the formal parameter types of MT<sub>S</sub>', and A<sub>1</sub>', ..., A<sub>n</sub>' are the type parameters of MT<sub>S</sub>', ‹V<sub>j</sub>`[`B<sub>1</sub>:=A<sub>1</sub>', ..., B<sub>n</sub>:=A<sub>n</sub>'`]` = U<sub>j</sub>'›

    

  - Let R<sub>S</sub> be the return type of MT<sub>S</sub>, and let R<sub>T</sub> be the return type of MT<sub>T</sub>. Then:


    - If R<sub>S</sub> mentions one of A<sub>1</sub>, ..., A<sub>n</sub>, and R<sub>T</sub> is not a proper type, *false*.

    - Otherwise, if `e`<sub>`i`</sub> is an explicitly typed lambda expression:


      - If R<sub>T</sub> is `void`, *true*.

      - Otherwise, if R<sub>S</sub> and R<sub>T</sub> are functional interface types, and `e`<sub>`i`</sub> has at least one result expression, then for each result expression in `e`<sub>`i`</sub>, this entire second step is repeated to infer constraints under which R<sub>S</sub> is more specific than R<sub>T</sub> θ' for the given result expression.

      - Otherwise, if R<sub>S</sub> is a primitive type and R<sub>T</sub> is not, and `e`<sub>`i`</sub> has at least one result expression, and each result expression of `e`<sub>`i`</sub> is a standalone expression ([§15.2](ch15-expressions.md#jls-15.2)) of a primitive type, *true*.

      - Otherwise, if R<sub>T</sub> is a primitive type and R<sub>S</sub> is not, and `e`<sub>`i`</sub> has at least one result expression, and each result expression of `e`<sub>`i`</sub> is either a standalone expression of a reference type or a poly expression, *true*.

      - Otherwise, ‹R<sub>S</sub> `<:` R<sub>T</sub> θ'›.

      

    - Otherwise, if `e`<sub>`i`</sub> is an exact method reference:


      - If R<sub>T</sub> is `void`, *true*.

      - Otherwise, if R<sub>S</sub> is a primitive type and R<sub>T</sub> is not, and the compile-time declaration for `e`<sub>`i`</sub> has a primitive return type, *true*.

      - Otherwise if R<sub>T</sub> is a primitive type and R<sub>S</sub> is not, and the compile-time declaration for `e`<sub>`i`</sub> has a reference return type, *true*.

      - Otherwise, ‹R<sub>S</sub> `<:` R<sub>T</sub> θ'›.

      

    - Otherwise, if `e`<sub>`i`</sub> is a parenthesized expression, these rules for constraints derived from R<sub>S</sub> and R<sub>T</sub> are applied recursively for the contained expression.

    - Otherwise, if `e`<sub>`i`</sub> is a conditional expression, these rules for constraints derived from R<sub>S</sub> and R<sub>T</sub> are applied recursively for each of the second and third operands.

    - Otherwise, if `e`<sub>`i`</sub> is a `switch` expression, these rules for constraints derived from R<sub>S</sub> and R<sub>T</sub> are applied recursively for each of its result expressions.

    - Otherwise, *false*.

    

  

- Third, if `m`<sub>`2`</sub> is applicable by variable arity invocation and has *k*+1 parameters, then where S<sub>k+1</sub> is the *k*+1'th variable arity parameter type of `m`<sub>`1`</sub> and T<sub>k+1</sub> is the result of θ applied to the *k*+1'th variable arity parameter type of `m`<sub>`2`</sub>, the constraint ‹S<sub>k+1</sub> `<:` T<sub>k+1</sub>› is generated.

- Fourth, the generated bounds and constraint formulas are reduced and incorporated with B to produce a bound set B'.

  If B' does not contain the bound *false*, and resolution of all the inference variables in B' succeeds, then `m`<sub>`1`</sub> is more specific than `m`<sub>`2`</sub>.

  Otherwise, `m`<sub>`1`</sub> is not more specific than `m`<sub>`2`</sub>.


### 18.5.5. Record Pattern Type Inference


When a record pattern ([§14.30.1](ch14-blocks-statements-patterns.md#jls-14.30.1)) for a generic record class R appears in a context in which values of a type T will be matched against it, and the pattern does not provide type arguments for R, the type arguments are inferred, as described below.


1.  If T is not checked cast convertible ([§5.5](ch05-conversions-contexts.md#jls-5.5)) to the raw type R, inference fails.

2.  Otherwise, where P<sub>1</sub>, ..., P<sub>n</sub> (*n* ≥ 1) are the type parameters of R, let α<sub>1</sub>, ..., α<sub>n</sub> be inference variables. An initial bound set, B<sub>0</sub>, is generated from the declared bounds of P<sub>1</sub>, ..., P<sub>n</sub>, as described in [§18.1.3](ch18-type-inference.md#jls-18.1.3).

3.  A type T' is derived from T, as follows:

    If T is a parameterized type, let T<sub>c</sub> be the result of capture conversion ([§5.1.10](ch05-conversions-contexts.md#jls-5.1.10)) applied to T, and let Z<sub>1</sub>, ..., Z<sub>k</sub> (*k* ≥ 0) be the type variables produced by capture that are type arguments in T<sub>c</sub>. (This includes type variables produced by the capture conversion in this step, and type variables produced by capture conversion elsewhere.) Let β<sub>1</sub>, ..., β<sub>k</sub> (*k* ≥ 0) be inference variables, and let θ be the substitution `[`Z<sub>1</sub>:=β<sub>1</sub>, ..., Z<sub>k</sub>:=β<sub>k</sub>`]`. T' is T<sub>c</sub> θ.

    Additional bounds for β<sub>1</sub>, ..., β<sub>k</sub> are incorporated into B<sub>0</sub> to form a bound set B<sub>1</sub>, as follows:


    - If β<sub>i</sub> (1 ≤ *i* ≤ *k*) replaced a type variable with an upper bound U, then the bound β<sub>i</sub> `<:` U θ appears in the bound set.

    - If β<sub>i</sub> (1 ≤ *i* ≤ *k*) replaced a type variable with a lower bound `L`, then the bound `L` θ `<:` β<sub>i</sub> appears in the bound set.

    - If no proper upper bounds otherwise exist for β<sub>i</sub> (1 ≤ *i* ≤ *k*), the bound β<sub>i</sub> `<:` `Object` appears in the bound set.

    - If T is any other class or interface type, then T' is the same as T, and B<sub>1</sub> is the same as B<sub>0</sub>.

    - If T is a type variable or an intersection type, then for each upper bound of the type variable or element of the intersection type, this step and step 4 are repeated recursively. All bounds produced in steps 3 and 4 are incorporated into a single bound set.

    

4.  If T' is a parameterization of a generic class G, and there exists a supertype of R`<`α<sub>1</sub>, ..., α<sub>n</sub>`>` that is also a parameterization of G, let R' be that supertype. The constraint formula ‹T'=R'› is reduced ([§18.2](ch18-type-inference.md#jls-18.2)) and the resulting bounds are incorporated into B<sub>1</sub> to produce a new bound set, B<sub>2</sub>.

    Otherwise, B<sub>2</sub> is the same as B<sub>1</sub>.

    If B<sub>2</sub> contains the bound *false*, inference fails.

5.  Otherwise, the inference variables α<sub>1</sub>, ..., α<sub>n</sub> are resolved in B<sub>2</sub> ([§18.4](ch18-type-inference.md#jls-18.4)). Unlike normal resolution, in this case resolution skips the step that attempts to produce an instantiation for an inference variable from its proper lower bounds or proper upper bounds; instead, any new instantiations are created by skipping directly to the step that introduces fresh type variables.

    If resolution fails, then inference fails.

6.  Otherwise, let A<sub>1</sub>, ..., A<sub>n</sub> be the resolved instantiations for α<sub>1</sub>, ..., α<sub>n</sub>, and let Y<sub>1</sub>, ..., Y<sub>p</sub> (*p* ≥ 0) be any fresh type variables introduced by resolution.

    The type of the record pattern is the upward projection of R`<`A<sub>1</sub>, ..., A<sub>n</sub>`>` with respect to Y<sub>1</sub>, ..., Y<sub>p</sub> ([§4.10.5](ch04-types-values-variables.md#jls-4.10.5)).


**Example 18.5.5-1. Record Pattern Type Inference**


The following program infers a parameterization for a record pattern:

``` programlisting

import java.util.function.UnaryOperator;

record Mapper<T>(T in, T out) implements UnaryOperator<T> {
    public T apply(T arg) {
        return in.equals(arg) ? out : null;
    }
}

class IllustrateRecordPatternTypeInference{
    void test(UnaryOperator<? extends CharSequence> op) {
        if (op instanceof Mapper(var in, var out)) {
            boolean shorter = out.length() < in.length();
        }
    }
}
```

In this case, R is the record class `Mapper`, and T is the type `UnaryOperator`\<`?` `extends` `CharSequence`\>. T is checked cast convertible to raw `Mapper`, so we'll infer an instantiation for α in `Mapper`\<α\>. T' is the type `UnaryOperator`\<β\>, where β has upper bound `CharSequence`.

`Mapper`\<α\> has the supertype `UnaryOperator`\<α\>, so we'll reduce the constraint formula ‹`UnaryOperator`\<β\>= `UnaryOperator`\<α\>›. This leads to the bound α=β. Incorporation further infers that α `<:` `CharSequence`.

Now we resolve α, yielding α = Y, a fresh type variable with upper bound `CharSequence`. Finally, we find the upward projection of `Mapper`\<Y\> with respect to Y, inferring that the type of the record pattern is `Mapper`\<`?` `extends` `CharSequence`\>.

Once we know the type of the record pattern, we can find its component types, which are matched against the component patterns of the record pattern. Pattern variables `in` and `out` both have type `CharSequence`.


  


