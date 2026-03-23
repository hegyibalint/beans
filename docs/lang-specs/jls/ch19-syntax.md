# Chapter 19. Syntax


This chapter repeats the syntactic grammar given in Chapters 4, 6-10, 14, and 15, as well as key parts of the lexical grammar from Chapter 3, using the notation from [§2.4](ch02-grammars.md#jls-2.4).


**Productions from [§3 (Lexical Structure)](ch03-lexical-structure.md)**


Identifier:


[IdentifierChars](ch03-lexical-structure.md#jls-IdentifierChars "IdentifierChars") but not a [ReservedKeyword](ch03-lexical-structure.md#jls-ReservedKeyword "ReservedKeyword") or [BooleanLiteral](ch03-lexical-structure.md#jls-BooleanLiteral "BooleanLiteral") or [NullLiteral](ch03-lexical-structure.md#jls-NullLiteral "NullLiteral")


IdentifierChars:


[JavaLetter](ch03-lexical-structure.md#jls-JavaLetter "JavaLetter") {[JavaLetterOrDigit](ch03-lexical-structure.md#jls-JavaLetterOrDigit "JavaLetterOrDigit")}


JavaLetter:


any Unicode character that is a "Java letter"


JavaLetterOrDigit:


any Unicode character that is a "Java letter-or-digit"


TypeIdentifier:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") but not `permits`, `record`, `sealed`, `var`, or `yield`


UnqualifiedMethodIdentifier:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") but not `yield`


Literal:


[IntegerLiteral](ch03-lexical-structure.md#jls-IntegerLiteral "IntegerLiteral")  
[FloatingPointLiteral](ch03-lexical-structure.md#jls-FloatingPointLiteral "FloatingPointLiteral")  
[BooleanLiteral](ch03-lexical-structure.md#jls-BooleanLiteral "BooleanLiteral")  
[CharacterLiteral](ch03-lexical-structure.md#jls-CharacterLiteral "CharacterLiteral")  
[StringLiteral](ch03-lexical-structure.md#jls-StringLiteral "StringLiteral")  
[TextBlock](ch03-lexical-structure.md#jls-TextBlock "TextBlock")  
[NullLiteral](ch03-lexical-structure.md#jls-NullLiteral "NullLiteral")


**Productions from [§4 (Types, Values, and Variables)](ch04-types-values-variables.md)**


Type:


[PrimitiveType](ch04-types-values-variables.md#jls-PrimitiveType "PrimitiveType")  
[ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType")


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


TypeParameter:


{[TypeParameterModifier](ch04-types-values-variables.md#jls-TypeParameterModifier "TypeParameterModifier")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeBound](ch04-types-values-variables.md#jls-TypeBound "TypeBound")\]


TypeParameterModifier:


[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")


TypeBound:


`extends` [TypeVariable](ch04-types-values-variables.md#jls-TypeVariable "TypeVariable")  
`extends` [ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType") {[AdditionalBound](ch04-types-values-variables.md#jls-AdditionalBound "AdditionalBound")}


AdditionalBound:


`&` [InterfaceType](ch04-types-values-variables.md#jls-InterfaceType "InterfaceType")


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


**Productions from [§6 (Names)](ch06-names.md)**


ModuleName:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[ModuleName](ch06-names.md#jls-ModuleName "ModuleName") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


PackageName:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[PackageName](ch06-names.md#jls-PackageName "PackageName") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


TypeName:


[TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier")  
[PackageOrTypeName](ch06-names.md#jls-PackageOrTypeName "PackageOrTypeName") `.` [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier")


ExpressionName:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[AmbiguousName](ch06-names.md#jls-AmbiguousName "AmbiguousName") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


MethodName:


[UnqualifiedMethodIdentifier](ch03-lexical-structure.md#jls-UnqualifiedMethodIdentifier "UnqualifiedMethodIdentifier")


PackageOrTypeName:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[PackageOrTypeName](ch06-names.md#jls-PackageOrTypeName "PackageOrTypeName") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


AmbiguousName:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[AmbiguousName](ch06-names.md#jls-AmbiguousName "AmbiguousName") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


**Productions from [§7 (Packages and Modules)](ch07-packages-modules.md)**


CompilationUnit:


[OrdinaryCompilationUnit](ch07-packages-modules.md#jls-OrdinaryCompilationUnit "OrdinaryCompilationUnit")  
[CompactCompilationUnit](ch07-packages-modules.md#jls-CompactCompilationUnit "CompactCompilationUnit")  
[ModularCompilationUnit](ch07-packages-modules.md#jls-ModularCompilationUnit "ModularCompilationUnit")


OrdinaryCompilationUnit:


\[[PackageDeclaration](ch07-packages-modules.md#jls-PackageDeclaration "PackageDeclaration")\] {[ImportDeclaration](ch07-packages-modules.md#jls-ImportDeclaration "ImportDeclaration")} {[TopLevelClassOrInterfaceDeclaration](ch07-packages-modules.md#jls-TopLevelClassOrInterfaceDeclaration "TopLevelClassOrInterfaceDeclaration")}


ModularCompilationUnit:


{[ImportDeclaration](ch07-packages-modules.md#jls-ImportDeclaration "ImportDeclaration")} [ModuleDeclaration](ch07-packages-modules.md#jls-ModuleDeclaration "ModuleDeclaration")


PackageDeclaration:


{[PackageModifier](ch07-packages-modules.md#jls-PackageModifier "PackageModifier")} `package` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") {`.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")} `;`


PackageModifier:


[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")


ImportDeclaration:


[SingleTypeImportDeclaration](ch07-packages-modules.md#jls-SingleTypeImportDeclaration "SingleTypeImportDeclaration")  
[TypeImportOnDemandDeclaration](ch07-packages-modules.md#jls-TypeImportOnDemandDeclaration "TypeImportOnDemandDeclaration")  
[SingleStaticImportDeclaration](ch07-packages-modules.md#jls-SingleStaticImportDeclaration "SingleStaticImportDeclaration")  
[StaticImportOnDemandDeclaration](ch07-packages-modules.md#jls-StaticImportOnDemandDeclaration "StaticImportOnDemandDeclaration")  
[SingleModuleImportDeclaration](ch07-packages-modules.md#jls-SingleModuleImportDeclaration "SingleModuleImportDeclaration")


SingleTypeImportDeclaration:


`import` [TypeName](ch06-names.md#jls-TypeName "TypeName") `;`


TypeImportOnDemandDeclaration:


`import` [PackageOrTypeName](ch06-names.md#jls-PackageOrTypeName "PackageOrTypeName") `.` `*` `;`


SingleStaticImportDeclaration:


`import` `static` [TypeName](ch06-names.md#jls-TypeName "TypeName") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `;`


StaticImportOnDemandDeclaration:


`import` `static` [TypeName](ch06-names.md#jls-TypeName "TypeName") `.` `*` `;`


TopLevelClassOrInterfaceDeclaration:


[ClassDeclaration](ch08-classes.md#jls-ClassDeclaration "ClassDeclaration")  
[InterfaceDeclaration](ch09-interfaces.md#jls-InterfaceDeclaration "InterfaceDeclaration")  
`;`


ModuleDeclaration:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} \[`open`\] `module` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") {`.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")} `{` {[ModuleDirective](ch07-packages-modules.md#jls-ModuleDirective "ModuleDirective")} `}`


ModuleDirective:


`requires` {[RequiresModifier](ch07-packages-modules.md#jls-RequiresModifier "RequiresModifier")} [ModuleName](ch06-names.md#jls-ModuleName "ModuleName") `;`  
`exports` [PackageName](ch06-names.md#jls-PackageName "PackageName") \[`to` [ModuleName](ch06-names.md#jls-ModuleName "ModuleName") {`,` [ModuleName](ch06-names.md#jls-ModuleName "ModuleName")}\] `;`  
`opens` [PackageName](ch06-names.md#jls-PackageName "PackageName") \[`to` [ModuleName](ch06-names.md#jls-ModuleName "ModuleName") {`,` [ModuleName](ch06-names.md#jls-ModuleName "ModuleName")}\] `;`  
`uses` [TypeName](ch06-names.md#jls-TypeName "TypeName") `;`  
`provides` [TypeName](ch06-names.md#jls-TypeName "TypeName") `with` [TypeName](ch06-names.md#jls-TypeName "TypeName") {`,` [TypeName](ch06-names.md#jls-TypeName "TypeName")} `;`


RequiresModifier:


(one of)  
`transitive` `static`


**Productions from [§8 (Classes)](ch08-classes.md)**


ClassDeclaration:


[NormalClassDeclaration](ch08-classes.md#jls-NormalClassDeclaration "NormalClassDeclaration")  
[EnumDeclaration](ch08-classes.md#jls-EnumDeclaration "EnumDeclaration")  
[RecordDeclaration](ch08-classes.md#jls-RecordDeclaration "RecordDeclaration")


NormalClassDeclaration:


{[ClassModifier](ch08-classes.md#jls-ClassModifier "ClassModifier")} `class` [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeParameters](ch08-classes.md#jls-TypeParameters "TypeParameters")\] \[[ClassExtends](ch08-classes.md#jls-ClassExtends "ClassExtends")\] \[[ClassImplements](ch08-classes.md#jls-ClassImplements "ClassImplements")\] \[[ClassPermits](ch08-classes.md#jls-ClassPermits "ClassPermits")\] [ClassBody](ch08-classes.md#jls-ClassBody "ClassBody")


ClassModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public` `protected` `private`  
`abstract` `static` `final` `sealed` `non-sealed` `strictfp`


TypeParameters:


`<` [TypeParameterList](ch08-classes.md#jls-TypeParameterList "TypeParameterList") `>`


TypeParameterList:


[TypeParameter](ch04-types-values-variables.md#jls-TypeParameter "TypeParameter") {`,` [TypeParameter](ch04-types-values-variables.md#jls-TypeParameter "TypeParameter")}


ClassExtends:


`extends` [ClassType](ch04-types-values-variables.md#jls-ClassType "ClassType")


ClassImplements:


`implements` [InterfaceTypeList](ch08-classes.md#jls-InterfaceTypeList "InterfaceTypeList")


InterfaceTypeList:


[InterfaceType](ch04-types-values-variables.md#jls-InterfaceType "InterfaceType") {`,` [InterfaceType](ch04-types-values-variables.md#jls-InterfaceType "InterfaceType")}


ClassPermits:


`permits` [TypeName](ch06-names.md#jls-TypeName "TypeName") {`,` [TypeName](ch06-names.md#jls-TypeName "TypeName")}


ClassBody:


`{` {[ClassBodyDeclaration](ch08-classes.md#jls-ClassBodyDeclaration "ClassBodyDeclaration")} `}`


ClassBodyDeclaration:


[ClassMemberDeclaration](ch08-classes.md#jls-ClassMemberDeclaration "ClassMemberDeclaration")  
[InstanceInitializer](ch08-classes.md#jls-InstanceInitializer "InstanceInitializer")  
[StaticInitializer](ch08-classes.md#jls-StaticInitializer "StaticInitializer")  
[ConstructorDeclaration](ch08-classes.md#jls-ConstructorDeclaration "ConstructorDeclaration")


ClassMemberDeclaration:


[FieldDeclaration](ch08-classes.md#jls-FieldDeclaration "FieldDeclaration")  
[MethodDeclaration](ch08-classes.md#jls-MethodDeclaration "MethodDeclaration")  
[ClassDeclaration](ch08-classes.md#jls-ClassDeclaration "ClassDeclaration")  
[InterfaceDeclaration](ch09-interfaces.md#jls-InterfaceDeclaration "InterfaceDeclaration")  
`;`


FieldDeclaration:


{[FieldModifier](ch08-classes.md#jls-FieldModifier "FieldModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") [VariableDeclaratorList](ch08-classes.md#jls-VariableDeclaratorList "VariableDeclaratorList") `;`


FieldModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public` `protected` `private`  
`static` `final` `transient` `volatile`


VariableDeclaratorList:


[VariableDeclarator](ch08-classes.md#jls-VariableDeclarator "VariableDeclarator") {`,` [VariableDeclarator](ch08-classes.md#jls-VariableDeclarator "VariableDeclarator")}


VariableDeclarator:


[VariableDeclaratorId](ch08-classes.md#jls-VariableDeclaratorId "VariableDeclaratorId") \[`=` [VariableInitializer](ch08-classes.md#jls-VariableInitializer "VariableInitializer")\]


VariableDeclaratorId:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") \[[Dims](ch04-types-values-variables.md#jls-Dims "Dims")\]  
`_`


VariableInitializer:


[Expression](ch15-expressions.md#jls-Expression "Expression")  
[ArrayInitializer](ch10-arrays.md#jls-ArrayInitializer "ArrayInitializer")


UnannType:


[UnannPrimitiveType](ch08-classes.md#jls-UnannPrimitiveType "UnannPrimitiveType")  
[UnannReferenceType](ch08-classes.md#jls-UnannReferenceType "UnannReferenceType")


UnannPrimitiveType:


[NumericType](ch04-types-values-variables.md#jls-NumericType "NumericType")  
`boolean`


UnannReferenceType:


[UnannClassOrInterfaceType](ch08-classes.md#jls-UnannClassOrInterfaceType "UnannClassOrInterfaceType")  
[UnannTypeVariable](ch08-classes.md#jls-UnannTypeVariable "UnannTypeVariable")  
[UnannArrayType](ch08-classes.md#jls-UnannArrayType "UnannArrayType")


UnannClassOrInterfaceType:


[UnannClassType](ch08-classes.md#jls-UnannClassType "UnannClassType")  
[UnannInterfaceType](ch08-classes.md#jls-UnannInterfaceType "UnannInterfaceType")


UnannClassType:


[TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\]  
[PackageName](ch06-names.md#jls-PackageName "PackageName") `.` {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\]  
[UnannClassOrInterfaceType](ch08-classes.md#jls-UnannClassOrInterfaceType "UnannClassOrInterfaceType") `.` {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\]


UnannInterfaceType:


[UnannClassType](ch08-classes.md#jls-UnannClassType "UnannClassType")


UnannTypeVariable:


[TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier")


UnannArrayType:


[UnannPrimitiveType](ch08-classes.md#jls-UnannPrimitiveType "UnannPrimitiveType") [Dims](ch04-types-values-variables.md#jls-Dims "Dims")  
[UnannClassOrInterfaceType](ch08-classes.md#jls-UnannClassOrInterfaceType "UnannClassOrInterfaceType") [Dims](ch04-types-values-variables.md#jls-Dims "Dims")  
[UnannTypeVariable](ch08-classes.md#jls-UnannTypeVariable "UnannTypeVariable") [Dims](ch04-types-values-variables.md#jls-Dims "Dims")


MethodDeclaration:


{[MethodModifier](ch08-classes.md#jls-MethodModifier "MethodModifier")} [MethodHeader](ch08-classes.md#jls-MethodHeader "MethodHeader") [MethodBody](ch08-classes.md#jls-MethodBody "MethodBody")


MethodModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public` `protected` `private`  
`abstract` `static` `final` `synchronized` `native` `strictfp`


MethodHeader:


[Result](ch08-classes.md#jls-Result "Result") [MethodDeclarator](ch08-classes.md#jls-MethodDeclarator "MethodDeclarator") \[[Throws](ch08-classes.md#jls-Throws "Throws")\]  
[TypeParameters](ch08-classes.md#jls-TypeParameters "TypeParameters") {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [Result](ch08-classes.md#jls-Result "Result") [MethodDeclarator](ch08-classes.md#jls-MethodDeclarator "MethodDeclarator") \[[Throws](ch08-classes.md#jls-Throws "Throws")\]


Result:


[UnannType](ch08-classes.md#jls-UnannType "UnannType")  
`void`


MethodDeclarator:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` \[[ReceiverParameter](ch08-classes.md#jls-ReceiverParameter "ReceiverParameter") `,`\] \[[FormalParameterList](ch08-classes.md#jls-FormalParameterList "FormalParameterList")\] `)` \[[Dims](ch04-types-values-variables.md#jls-Dims "Dims")\]


ReceiverParameter:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") \[[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `.`\] `this`


FormalParameterList:


[FormalParameter](ch08-classes.md#jls-FormalParameter "FormalParameter") {`,` [FormalParameter](ch08-classes.md#jls-FormalParameter "FormalParameter")}


FormalParameter:


{[VariableModifier](ch08-classes.md#jls-VariableModifier "VariableModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") [VariableDeclaratorId](ch08-classes.md#jls-VariableDeclaratorId "VariableDeclaratorId")  
[VariableArityParameter](ch08-classes.md#jls-VariableArityParameter "VariableArityParameter")


VariableArityParameter:


{[VariableModifier](ch08-classes.md#jls-VariableModifier "VariableModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `...` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


VariableModifier:


[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")  
`final`


Throws:


`throws` [ExceptionTypeList](ch08-classes.md#jls-ExceptionTypeList "ExceptionTypeList")


ExceptionTypeList:


[ExceptionType](ch08-classes.md#jls-ExceptionType "ExceptionType") {`,` [ExceptionType](ch08-classes.md#jls-ExceptionType "ExceptionType")}


ExceptionType:


[ClassType](ch04-types-values-variables.md#jls-ClassType "ClassType")  
[TypeVariable](ch04-types-values-variables.md#jls-TypeVariable "TypeVariable")


MethodBody:


[Block](ch14-blocks-statements-patterns.md#jls-Block "Block")  
`;`


InstanceInitializer:


[Block](ch14-blocks-statements-patterns.md#jls-Block "Block")


StaticInitializer:


`static` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block")


ConstructorDeclaration:


{[ConstructorModifier](ch08-classes.md#jls-ConstructorModifier "ConstructorModifier")} [ConstructorDeclarator](ch08-classes.md#jls-ConstructorDeclarator "ConstructorDeclarator") \[[Throws](ch08-classes.md#jls-Throws "Throws")\] [ConstructorBody](ch08-classes.md#jls-ConstructorBody "ConstructorBody")


ConstructorModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public` `protected` `private`


ConstructorDeclarator:


\[[TypeParameters](ch08-classes.md#jls-TypeParameters "TypeParameters")\] [SimpleTypeName](ch08-classes.md#jls-SimpleTypeName "SimpleTypeName") `(` \[[ReceiverParameter](ch08-classes.md#jls-ReceiverParameter "ReceiverParameter") `,`\] \[[FormalParameterList](ch08-classes.md#jls-FormalParameterList "FormalParameterList")\] `)`


SimpleTypeName:


[TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier")


ConstructorBody:


`{` \[[BlockStatements](ch14-blocks-statements-patterns.md#jls-BlockStatements "BlockStatements")\] [ConstructorInvocation](ch08-classes.md#jls-ConstructorInvocation "ConstructorInvocation") \[[BlockStatements](ch14-blocks-statements-patterns.md#jls-BlockStatements "BlockStatements")\] `}`  
`{` \[[BlockStatements](ch14-blocks-statements-patterns.md#jls-BlockStatements "BlockStatements")\] `}`


ConstructorInvocation:


\[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] `this` `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)` `;`  
\[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] `super` `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)` `;`  
[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName") `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] `super` `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)` `;`  
[Primary](ch15-expressions.md#jls-Primary "Primary") `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] `super` `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)` `;`


EnumDeclaration:


{[ClassModifier](ch08-classes.md#jls-ClassModifier "ClassModifier")} `enum` [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[ClassImplements](ch08-classes.md#jls-ClassImplements "ClassImplements")\] [EnumBody](ch08-classes.md#jls-EnumBody "EnumBody")


EnumBody:


`{` \[[EnumConstantList](ch08-classes.md#jls-EnumConstantList "EnumConstantList")\] \[`,`\] \[[EnumBodyDeclarations](ch08-classes.md#jls-EnumBodyDeclarations "EnumBodyDeclarations")\] `}`


EnumConstantList:


[EnumConstant](ch08-classes.md#jls-EnumConstant "EnumConstant") {`,` [EnumConstant](ch08-classes.md#jls-EnumConstant "EnumConstant")}


EnumConstant:


{[EnumConstantModifier](ch08-classes.md#jls-EnumConstantModifier "EnumConstantModifier")} [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") \[`(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`\] \[[ClassBody](ch08-classes.md#jls-ClassBody "ClassBody")\]


EnumConstantModifier:


[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")


EnumBodyDeclarations:


`;` {[ClassBodyDeclaration](ch08-classes.md#jls-ClassBodyDeclaration "ClassBodyDeclaration")}


RecordDeclaration:


{[ClassModifier](ch08-classes.md#jls-ClassModifier "ClassModifier")} `record` [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeParameters](ch08-classes.md#jls-TypeParameters "TypeParameters")\] [RecordHeader](ch08-classes.md#jls-RecordHeader "RecordHeader") \[[ClassImplements](ch08-classes.md#jls-ClassImplements "ClassImplements")\] [RecordBody](ch08-classes.md#jls-RecordBody "RecordBody")


RecordHeader:


`(` \[[RecordComponentList](ch08-classes.md#jls-RecordComponentList "RecordComponentList")\] `)`


RecordComponentList:


[RecordComponent](ch08-classes.md#jls-RecordComponent "RecordComponent") {`,` [RecordComponent](ch08-classes.md#jls-RecordComponent "RecordComponent")}


RecordComponent:


{[RecordComponentModifier](ch08-classes.md#jls-RecordComponentModifier "RecordComponentModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[VariableArityRecordComponent](ch08-classes.md#jls-VariableArityRecordComponent "VariableArityRecordComponent")


VariableArityRecordComponent:


{[RecordComponentModifier](ch08-classes.md#jls-RecordComponentModifier "RecordComponentModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `...` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


RecordComponentModifier:


[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")


RecordBody:


`{` {[RecordBodyDeclaration](ch08-classes.md#jls-RecordBodyDeclaration "RecordBodyDeclaration")} `}`


RecordBodyDeclaration:


[ClassBodyDeclaration](ch08-classes.md#jls-ClassBodyDeclaration "ClassBodyDeclaration")  
[CompactConstructorDeclaration](ch08-classes.md#jls-CompactConstructorDeclaration "CompactConstructorDeclaration")


CompactConstructorDeclaration:


{[ConstructorModifier](ch08-classes.md#jls-ConstructorModifier "ConstructorModifier")} [SimpleTypeName](ch08-classes.md#jls-SimpleTypeName "SimpleTypeName") [ConstructorBody](ch08-classes.md#jls-ConstructorBody "ConstructorBody")


**Productions from [§9 (Interfaces)](ch09-interfaces.md)**


InterfaceDeclaration:


[NormalInterfaceDeclaration](ch09-interfaces.md#jls-NormalInterfaceDeclaration "NormalInterfaceDeclaration")  
[AnnotationInterfaceDeclaration](ch09-interfaces.md#jls-AnnotationInterfaceDeclaration "AnnotationInterfaceDeclaration")


NormalInterfaceDeclaration:


{[InterfaceModifier](ch09-interfaces.md#jls-InterfaceModifier "InterfaceModifier")} `interface` [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") \[[TypeParameters](ch08-classes.md#jls-TypeParameters "TypeParameters")\] \[[InterfaceExtends](ch09-interfaces.md#jls-InterfaceExtends "InterfaceExtends")\] \[[InterfacePermits](ch09-interfaces.md#jls-InterfacePermits "InterfacePermits")\] [InterfaceBody](ch09-interfaces.md#jls-InterfaceBody "InterfaceBody")


InterfaceModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public` `protected` `private`  
`abstract` `static` `sealed` `non-sealed` `strictfp`


InterfaceExtends:


`extends` [InterfaceTypeList](ch08-classes.md#jls-InterfaceTypeList "InterfaceTypeList")


InterfacePermits:


`permits` [TypeName](ch06-names.md#jls-TypeName "TypeName") {`,` [TypeName](ch06-names.md#jls-TypeName "TypeName")}


InterfaceBody:


`{` {[InterfaceMemberDeclaration](ch09-interfaces.md#jls-InterfaceMemberDeclaration "InterfaceMemberDeclaration")} `}`


InterfaceMemberDeclaration:


[ConstantDeclaration](ch09-interfaces.md#jls-ConstantDeclaration "ConstantDeclaration")  
[InterfaceMethodDeclaration](ch09-interfaces.md#jls-InterfaceMethodDeclaration "InterfaceMethodDeclaration")  
[ClassDeclaration](ch08-classes.md#jls-ClassDeclaration "ClassDeclaration")  
[InterfaceDeclaration](ch09-interfaces.md#jls-InterfaceDeclaration "InterfaceDeclaration")  
`;`


ConstantDeclaration:


{[ConstantModifier](ch09-interfaces.md#jls-ConstantModifier "ConstantModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") [VariableDeclaratorList](ch08-classes.md#jls-VariableDeclaratorList "VariableDeclaratorList") `;`


ConstantModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public`  
`static` `final`


InterfaceMethodDeclaration:


{[InterfaceMethodModifier](ch09-interfaces.md#jls-InterfaceMethodModifier "InterfaceMethodModifier")} [MethodHeader](ch08-classes.md#jls-MethodHeader "MethodHeader") [MethodBody](ch08-classes.md#jls-MethodBody "MethodBody")


InterfaceMethodModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public` `private`  
`abstract` `default` `static` `strictfp`


AnnotationInterfaceDeclaration:


{[InterfaceModifier](ch09-interfaces.md#jls-InterfaceModifier "InterfaceModifier")} `@` `interface` [TypeIdentifier](ch03-lexical-structure.md#jls-TypeIdentifier "TypeIdentifier") [AnnotationInterfaceBody](ch09-interfaces.md#jls-AnnotationInterfaceBody "AnnotationInterfaceBody")


AnnotationInterfaceBody:


`{` {[AnnotationInterfaceMemberDeclaration](ch09-interfaces.md#jls-AnnotationInterfaceMemberDeclaration "AnnotationInterfaceMemberDeclaration")} `}`


AnnotationInterfaceMemberDeclaration:


[AnnotationInterfaceElementDeclaration](ch09-interfaces.md#jls-AnnotationInterfaceElementDeclaration "AnnotationInterfaceElementDeclaration")  
[ConstantDeclaration](ch09-interfaces.md#jls-ConstantDeclaration "ConstantDeclaration")  
[ClassDeclaration](ch08-classes.md#jls-ClassDeclaration "ClassDeclaration")  
[InterfaceDeclaration](ch09-interfaces.md#jls-InterfaceDeclaration "InterfaceDeclaration")  
`;`


AnnotationInterfaceElementDeclaration:


{[AnnotationInterfaceElementModifier](ch09-interfaces.md#jls-AnnotationInterfaceElementModifier "AnnotationInterfaceElementModifier")} [UnannType](ch08-classes.md#jls-UnannType "UnannType") [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` `)` \[[Dims](ch04-types-values-variables.md#jls-Dims "Dims")\] \[[DefaultValue](ch09-interfaces.md#jls-DefaultValue "DefaultValue")\] `;`


AnnotationInterfaceElementModifier:


(one of)  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation") `public`  
`abstract`


DefaultValue:


`default` [ElementValue](ch09-interfaces.md#jls-ElementValue "ElementValue")


Annotation:


[NormalAnnotation](ch09-interfaces.md#jls-NormalAnnotation "NormalAnnotation")  
[MarkerAnnotation](ch09-interfaces.md#jls-MarkerAnnotation "MarkerAnnotation")  
[SingleElementAnnotation](ch09-interfaces.md#jls-SingleElementAnnotation "SingleElementAnnotation")


NormalAnnotation:


`@` [TypeName](ch06-names.md#jls-TypeName "TypeName") `(` \[[ElementValuePairList](ch09-interfaces.md#jls-ElementValuePairList "ElementValuePairList")\] `)`


ElementValuePairList:


[ElementValuePair](ch09-interfaces.md#jls-ElementValuePair "ElementValuePair") {`,` [ElementValuePair](ch09-interfaces.md#jls-ElementValuePair "ElementValuePair")}


ElementValuePair:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `=` [ElementValue](ch09-interfaces.md#jls-ElementValue "ElementValue")


ElementValue:


[ConditionalExpression](ch15-expressions.md#jls-ConditionalExpression "ConditionalExpression")  
[ElementValueArrayInitializer](ch09-interfaces.md#jls-ElementValueArrayInitializer "ElementValueArrayInitializer")  
[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")


ElementValueArrayInitializer:


`{` \[[ElementValueList](ch09-interfaces.md#jls-ElementValueList "ElementValueList")\] \[`,`\] `}`


ElementValueList:


[ElementValue](ch09-interfaces.md#jls-ElementValue "ElementValue") {`,` [ElementValue](ch09-interfaces.md#jls-ElementValue "ElementValue")}


MarkerAnnotation:


`@` [TypeName](ch06-names.md#jls-TypeName "TypeName")


SingleElementAnnotation:


`@` [TypeName](ch06-names.md#jls-TypeName "TypeName") `(` [ElementValue](ch09-interfaces.md#jls-ElementValue "ElementValue") `)`


**Productions from [§10 (Arrays)](ch10-arrays.md)**


ArrayInitializer:


`{` \[[VariableInitializerList](ch10-arrays.md#jls-VariableInitializerList "VariableInitializerList")\] \[`,`\] `}`


VariableInitializerList:


[VariableInitializer](ch08-classes.md#jls-VariableInitializer "VariableInitializer") {`,` [VariableInitializer](ch08-classes.md#jls-VariableInitializer "VariableInitializer")}


**Productions from [§14 (Blocks, Statements, and Patterns)](ch14-blocks-statements-patterns.md)**


Block:


`{` \[[BlockStatements](ch14-blocks-statements-patterns.md#jls-BlockStatements "BlockStatements")\] `}`


BlockStatements:


[BlockStatement](ch14-blocks-statements-patterns.md#jls-BlockStatement "BlockStatement") {[BlockStatement](ch14-blocks-statements-patterns.md#jls-BlockStatement "BlockStatement")}


BlockStatement:


[LocalClassOrInterfaceDeclaration](ch14-blocks-statements-patterns.md#jls-LocalClassOrInterfaceDeclaration "LocalClassOrInterfaceDeclaration")  
[LocalVariableDeclarationStatement](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclarationStatement "LocalVariableDeclarationStatement")  
[Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


LocalClassOrInterfaceDeclaration:


[ClassDeclaration](ch08-classes.md#jls-ClassDeclaration "ClassDeclaration")  
[NormalInterfaceDeclaration](ch09-interfaces.md#jls-NormalInterfaceDeclaration "NormalInterfaceDeclaration")


LocalVariableDeclarationStatement:


[LocalVariableDeclaration](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclaration "LocalVariableDeclaration") `;`


LocalVariableDeclaration:


{[VariableModifier](ch08-classes.md#jls-VariableModifier "VariableModifier")} [LocalVariableType](ch14-blocks-statements-patterns.md#jls-LocalVariableType "LocalVariableType") [VariableDeclaratorList](ch08-classes.md#jls-VariableDeclaratorList "VariableDeclaratorList")


LocalVariableType:


[UnannType](ch08-classes.md#jls-UnannType "UnannType")  
`var`


Statement:


[StatementWithoutTrailingSubstatement](ch14-blocks-statements-patterns.md#jls-StatementWithoutTrailingSubstatement "StatementWithoutTrailingSubstatement")  
[LabeledStatement](ch14-blocks-statements-patterns.md#jls-LabeledStatement "LabeledStatement")  
[IfThenStatement](ch14-blocks-statements-patterns.md#jls-IfThenStatement "IfThenStatement")  
[IfThenElseStatement](ch14-blocks-statements-patterns.md#jls-IfThenElseStatement "IfThenElseStatement")  
[WhileStatement](ch14-blocks-statements-patterns.md#jls-WhileStatement "WhileStatement")  
[ForStatement](ch14-blocks-statements-patterns.md#jls-ForStatement "ForStatement")


StatementNoShortIf:


[StatementWithoutTrailingSubstatement](ch14-blocks-statements-patterns.md#jls-StatementWithoutTrailingSubstatement "StatementWithoutTrailingSubstatement")  
[LabeledStatementNoShortIf](ch14-blocks-statements-patterns.md#jls-LabeledStatementNoShortIf "LabeledStatementNoShortIf")  
[IfThenElseStatementNoShortIf](ch14-blocks-statements-patterns.md#jls-IfThenElseStatementNoShortIf "IfThenElseStatementNoShortIf")  
[WhileStatementNoShortIf](ch14-blocks-statements-patterns.md#jls-WhileStatementNoShortIf "WhileStatementNoShortIf")  
[ForStatementNoShortIf](ch14-blocks-statements-patterns.md#jls-ForStatementNoShortIf "ForStatementNoShortIf")


StatementWithoutTrailingSubstatement:


[Block](ch14-blocks-statements-patterns.md#jls-Block "Block")  
[EmptyStatement](ch14-blocks-statements-patterns.md#jls-EmptyStatement "EmptyStatement")  
[ExpressionStatement](ch14-blocks-statements-patterns.md#jls-ExpressionStatement "ExpressionStatement")  
[AssertStatement](ch14-blocks-statements-patterns.md#jls-AssertStatement "AssertStatement")  
[SwitchStatement](ch14-blocks-statements-patterns.md#jls-SwitchStatement "SwitchStatement")  
[DoStatement](ch14-blocks-statements-patterns.md#jls-DoStatement "DoStatement")  
[BreakStatement](ch14-blocks-statements-patterns.md#jls-BreakStatement "BreakStatement")  
[ContinueStatement](ch14-blocks-statements-patterns.md#jls-ContinueStatement "ContinueStatement")  
[ReturnStatement](ch14-blocks-statements-patterns.md#jls-ReturnStatement "ReturnStatement")  
[SynchronizedStatement](ch14-blocks-statements-patterns.md#jls-SynchronizedStatement "SynchronizedStatement")  
[ThrowStatement](ch14-blocks-statements-patterns.md#jls-ThrowStatement "ThrowStatement")  
[TryStatement](ch14-blocks-statements-patterns.md#jls-TryStatement "TryStatement")  
[YieldStatement](ch14-blocks-statements-patterns.md#jls-YieldStatement "YieldStatement")


EmptyStatement:


`;`


LabeledStatement:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `:` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


LabeledStatementNoShortIf:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `:` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf")


ExpressionStatement:


[StatementExpression](ch14-blocks-statements-patterns.md#jls-StatementExpression "StatementExpression") `;`


StatementExpression:


[Assignment](ch15-expressions.md#jls-Assignment "Assignment")  
[PreIncrementExpression](ch15-expressions.md#jls-PreIncrementExpression "PreIncrementExpression")  
[PreDecrementExpression](ch15-expressions.md#jls-PreDecrementExpression "PreDecrementExpression")  
[PostIncrementExpression](ch15-expressions.md#jls-PostIncrementExpression "PostIncrementExpression")  
[PostDecrementExpression](ch15-expressions.md#jls-PostDecrementExpression "PostDecrementExpression")  
[MethodInvocation](ch15-expressions.md#jls-MethodInvocation "MethodInvocation")  
[ClassInstanceCreationExpression](ch15-expressions.md#jls-ClassInstanceCreationExpression "ClassInstanceCreationExpression")


IfThenStatement:


`if` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


IfThenElseStatement:


`if` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf") `else` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


IfThenElseStatementNoShortIf:


`if` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf") `else` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf")


AssertStatement:


`assert` [Expression](ch15-expressions.md#jls-Expression "Expression") `;`  
`assert` [Expression](ch15-expressions.md#jls-Expression "Expression") `:` [Expression](ch15-expressions.md#jls-Expression "Expression") `;`


SwitchStatement:


`switch` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [SwitchBlock](ch14-blocks-statements-patterns.md#jls-SwitchBlock "SwitchBlock")


SwitchBlock:


`{` [SwitchRule](ch14-blocks-statements-patterns.md#jls-SwitchRule "SwitchRule") {[SwitchRule](ch14-blocks-statements-patterns.md#jls-SwitchRule "SwitchRule")} `}`  
`{` {[SwitchBlockStatementGroup](ch14-blocks-statements-patterns.md#jls-SwitchBlockStatementGroup "SwitchBlockStatementGroup")} {[SwitchLabel](ch14-blocks-statements-patterns.md#jls-SwitchLabel "SwitchLabel") `:`} `}`


SwitchRule:


[SwitchLabel](ch14-blocks-statements-patterns.md#jls-SwitchLabel "SwitchLabel") `->` [Expression](ch15-expressions.md#jls-Expression "Expression") `;`  
[SwitchLabel](ch14-blocks-statements-patterns.md#jls-SwitchLabel "SwitchLabel") `->` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block")  
[SwitchLabel](ch14-blocks-statements-patterns.md#jls-SwitchLabel "SwitchLabel") `->` [ThrowStatement](ch14-blocks-statements-patterns.md#jls-ThrowStatement "ThrowStatement")


SwitchBlockStatementGroup:


[SwitchLabel](ch14-blocks-statements-patterns.md#jls-SwitchLabel "SwitchLabel") `:` {[SwitchLabel](ch14-blocks-statements-patterns.md#jls-SwitchLabel "SwitchLabel") `:`} [BlockStatements](ch14-blocks-statements-patterns.md#jls-BlockStatements "BlockStatements")


SwitchLabel:


`case` [CaseConstant](ch14-blocks-statements-patterns.md#jls-CaseConstant "CaseConstant") {`,` [CaseConstant](ch14-blocks-statements-patterns.md#jls-CaseConstant "CaseConstant")}  
`case` `null` \[`,` `default`\]  
`case` [CasePattern](ch14-blocks-statements-patterns.md#jls-CasePattern "CasePattern") {`,` [CasePattern](ch14-blocks-statements-patterns.md#jls-CasePattern "CasePattern")} \[[Guard](ch14-blocks-statements-patterns.md#jls-Guard "Guard")\]  
`default`


CaseConstant:


[ConditionalExpression](ch15-expressions.md#jls-ConditionalExpression "ConditionalExpression")


CasePattern:


[Pattern](ch14-blocks-statements-patterns.md#jls-Pattern "Pattern")


Guard:


`when` [Expression](ch15-expressions.md#jls-Expression "Expression")


WhileStatement:


`while` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


WhileStatementNoShortIf:


`while` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf")


DoStatement:


`do` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement") `while` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` `;`


ForStatement:


[BasicForStatement](ch14-blocks-statements-patterns.md#jls-BasicForStatement "BasicForStatement")  
[EnhancedForStatement](ch14-blocks-statements-patterns.md#jls-EnhancedForStatement "EnhancedForStatement")


ForStatementNoShortIf:


[BasicForStatementNoShortIf](ch14-blocks-statements-patterns.md#jls-BasicForStatementNoShortIf "BasicForStatementNoShortIf")  
[EnhancedForStatementNoShortIf](ch14-blocks-statements-patterns.md#jls-EnhancedForStatementNoShortIf "EnhancedForStatementNoShortIf")


BasicForStatement:


`for` `(` \[[ForInit](ch14-blocks-statements-patterns.md#jls-ForInit "ForInit")\] `;` \[[Expression](ch15-expressions.md#jls-Expression "Expression")\] `;` \[[ForUpdate](ch14-blocks-statements-patterns.md#jls-ForUpdate "ForUpdate")\] `)` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


BasicForStatementNoShortIf:


`for` `(` \[[ForInit](ch14-blocks-statements-patterns.md#jls-ForInit "ForInit")\] `;` \[[Expression](ch15-expressions.md#jls-Expression "Expression")\] `;` \[[ForUpdate](ch14-blocks-statements-patterns.md#jls-ForUpdate "ForUpdate")\] `)` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf")


ForInit:


[StatementExpressionList](ch14-blocks-statements-patterns.md#jls-StatementExpressionList "StatementExpressionList")  
[LocalVariableDeclaration](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclaration "LocalVariableDeclaration")


ForUpdate:


[StatementExpressionList](ch14-blocks-statements-patterns.md#jls-StatementExpressionList "StatementExpressionList")


StatementExpressionList:


[StatementExpression](ch14-blocks-statements-patterns.md#jls-StatementExpression "StatementExpression") {`,` [StatementExpression](ch14-blocks-statements-patterns.md#jls-StatementExpression "StatementExpression")}


EnhancedForStatement:


`for` `(` [LocalVariableDeclaration](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclaration "LocalVariableDeclaration") `:` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [Statement](ch14-blocks-statements-patterns.md#jls-Statement "Statement")


EnhancedForStatementNoShortIf:


`for` `(` [LocalVariableDeclaration](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclaration "LocalVariableDeclaration") `:` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [StatementNoShortIf](ch14-blocks-statements-patterns.md#jls-StatementNoShortIf "StatementNoShortIf")


BreakStatement:


`break` \[[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")\] `;`


YieldStatement:


`yield` [Expression](ch15-expressions.md#jls-Expression "Expression") `;`


ContinueStatement:


`continue` \[[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")\] `;`


ReturnStatement:


`return` \[[Expression](ch15-expressions.md#jls-Expression "Expression")\] `;`


ThrowStatement:


`throw` [Expression](ch15-expressions.md#jls-Expression "Expression") `;`


SynchronizedStatement:


`synchronized` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block")


TryStatement:


`try` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block") [Catches](ch14-blocks-statements-patterns.md#jls-Catches "Catches")  
`try` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block") \[[Catches](ch14-blocks-statements-patterns.md#jls-Catches "Catches")\] [Finally](ch14-blocks-statements-patterns.md#jls-Finally "Finally")  
[TryWithResourcesStatement](ch14-blocks-statements-patterns.md#jls-TryWithResourcesStatement "TryWithResourcesStatement")


Catches:


[CatchClause](ch14-blocks-statements-patterns.md#jls-CatchClause "CatchClause") {[CatchClause](ch14-blocks-statements-patterns.md#jls-CatchClause "CatchClause")}


CatchClause:


`catch` `(` [CatchFormalParameter](ch14-blocks-statements-patterns.md#jls-CatchFormalParameter "CatchFormalParameter") `)` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block")


CatchFormalParameter:


{[VariableModifier](ch08-classes.md#jls-VariableModifier "VariableModifier")} [CatchType](ch14-blocks-statements-patterns.md#jls-CatchType "CatchType") [VariableDeclaratorId](ch08-classes.md#jls-VariableDeclaratorId "VariableDeclaratorId")


CatchType:


[UnannClassType](ch08-classes.md#jls-UnannClassType "UnannClassType") {`|` [ClassType](ch04-types-values-variables.md#jls-ClassType "ClassType")}


Finally:


`finally` [Block](ch14-blocks-statements-patterns.md#jls-Block "Block")


TryWithResourcesStatement:


`try` [ResourceSpecification](ch14-blocks-statements-patterns.md#jls-ResourceSpecification "ResourceSpecification") [Block](ch14-blocks-statements-patterns.md#jls-Block "Block") \[[Catches](ch14-blocks-statements-patterns.md#jls-Catches "Catches")\] \[[Finally](ch14-blocks-statements-patterns.md#jls-Finally "Finally")\]


ResourceSpecification:


`(` [ResourceList](ch14-blocks-statements-patterns.md#jls-ResourceList "ResourceList") \[`;`\] `)`


ResourceList:


[Resource](ch14-blocks-statements-patterns.md#jls-Resource "Resource") {`;` [Resource](ch14-blocks-statements-patterns.md#jls-Resource "Resource")}


Resource:


[LocalVariableDeclaration](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclaration "LocalVariableDeclaration")  
[VariableAccess](ch14-blocks-statements-patterns.md#jls-VariableAccess "VariableAccess")


VariableAccess:


[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName")  
[FieldAccess](ch15-expressions.md#jls-FieldAccess "FieldAccess")


Pattern:


[TypePattern](ch14-blocks-statements-patterns.md#jls-TypePattern "TypePattern")  
[RecordPattern](ch14-blocks-statements-patterns.md#jls-RecordPattern "RecordPattern")


TypePattern:


[LocalVariableDeclaration](ch14-blocks-statements-patterns.md#jls-LocalVariableDeclaration "LocalVariableDeclaration")


RecordPattern:


[ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType") `(` \[[ComponentPatternList](ch14-blocks-statements-patterns.md#jls-ComponentPatternList "ComponentPatternList")\] `)`


ComponentPatternList:


[ComponentPattern](ch14-blocks-statements-patterns.md#jls-ComponentPattern "ComponentPattern") {`,` [ComponentPattern](ch14-blocks-statements-patterns.md#jls-ComponentPattern "ComponentPattern") }


ComponentPattern:


[Pattern](ch14-blocks-statements-patterns.md#jls-Pattern "Pattern")  
[MatchAllPattern](ch14-blocks-statements-patterns.md#jls-MatchAllPattern "MatchAllPattern")


MatchAllPattern:


`_`


**Productions from [§15 (Expressions)](ch15-expressions.md)**


Primary:


[PrimaryNoNewArray](ch15-expressions.md#jls-PrimaryNoNewArray "PrimaryNoNewArray")  
[ArrayCreationExpression](ch15-expressions.md#jls-ArrayCreationExpression "ArrayCreationExpression")


PrimaryNoNewArray:


[Literal](ch03-lexical-structure.md#jls-Literal "Literal")  
[ClassLiteral](ch15-expressions.md#jls-ClassLiteral "ClassLiteral")  
`this`  
[TypeName](ch06-names.md#jls-TypeName "TypeName") `.` `this`  
`(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)`  
[ClassInstanceCreationExpression](ch15-expressions.md#jls-ClassInstanceCreationExpression "ClassInstanceCreationExpression")  
[FieldAccess](ch15-expressions.md#jls-FieldAccess "FieldAccess")  
[ArrayAccess](ch15-expressions.md#jls-ArrayAccess "ArrayAccess")  
[MethodInvocation](ch15-expressions.md#jls-MethodInvocation "MethodInvocation")  
[MethodReference](ch15-expressions.md#jls-MethodReference "MethodReference")


ClassLiteral:


[TypeName](ch06-names.md#jls-TypeName "TypeName") {`[` `]`} `.` `class`  
[NumericType](ch04-types-values-variables.md#jls-NumericType "NumericType") {`[` `]`} `.` `class`  
`boolean` {`[` `]`} `.` `class`  
`void` `.` `class`


ClassInstanceCreationExpression:


[UnqualifiedClassInstanceCreationExpression](ch15-expressions.md#jls-UnqualifiedClassInstanceCreationExpression "UnqualifiedClassInstanceCreationExpression")  
[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName") `.` [UnqualifiedClassInstanceCreationExpression](ch15-expressions.md#jls-UnqualifiedClassInstanceCreationExpression "UnqualifiedClassInstanceCreationExpression")  
[Primary](ch15-expressions.md#jls-Primary "Primary") `.` [UnqualifiedClassInstanceCreationExpression](ch15-expressions.md#jls-UnqualifiedClassInstanceCreationExpression "UnqualifiedClassInstanceCreationExpression")


UnqualifiedClassInstanceCreationExpression:


`new` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [ClassOrInterfaceTypeToInstantiate](ch15-expressions.md#jls-ClassOrInterfaceTypeToInstantiate "ClassOrInterfaceTypeToInstantiate") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)` \[[ClassBody](ch08-classes.md#jls-ClassBody "ClassBody")\]


ClassOrInterfaceTypeToInstantiate:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") {`.` {[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")} \[[TypeArgumentsOrDiamond](ch15-expressions.md#jls-TypeArgumentsOrDiamond "TypeArgumentsOrDiamond")\]


TypeArgumentsOrDiamond:


[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")  
`<>`


ArrayCreationExpression:


[ArrayCreationExpressionWithoutInitializer](ch15-expressions.md#jls-ArrayCreationExpressionWithoutInitializer "ArrayCreationExpressionWithoutInitializer")  
[ArrayCreationExpressionWithInitializer](ch15-expressions.md#jls-ArrayCreationExpressionWithInitializer "ArrayCreationExpressionWithInitializer")


ArrayCreationExpressionWithoutInitializer:


`new` [PrimitiveType](ch04-types-values-variables.md#jls-PrimitiveType "PrimitiveType") [DimExprs](ch15-expressions.md#jls-DimExprs "DimExprs") \[[Dims](ch04-types-values-variables.md#jls-Dims "Dims")\]  
`new` [ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType") [DimExprs](ch15-expressions.md#jls-DimExprs "DimExprs") \[[Dims](ch04-types-values-variables.md#jls-Dims "Dims")\]


ArrayCreationExpressionWithInitializer:


`new` [PrimitiveType](ch04-types-values-variables.md#jls-PrimitiveType "PrimitiveType") [Dims](ch04-types-values-variables.md#jls-Dims "Dims") [ArrayInitializer](ch10-arrays.md#jls-ArrayInitializer "ArrayInitializer")  
`new` [ClassOrInterfaceType](ch04-types-values-variables.md#jls-ClassOrInterfaceType "ClassOrInterfaceType") [Dims](ch04-types-values-variables.md#jls-Dims "Dims") [ArrayInitializer](ch10-arrays.md#jls-ArrayInitializer "ArrayInitializer")


DimExprs:


[DimExpr](ch15-expressions.md#jls-DimExpr "DimExpr") {[DimExpr](ch15-expressions.md#jls-DimExpr "DimExpr")}


DimExpr:


{[Annotation](ch09-interfaces.md#jls-Annotation "Annotation")} `[` [Expression](ch15-expressions.md#jls-Expression "Expression") `]`


ArrayAccess:


[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName") `[` [Expression](ch15-expressions.md#jls-Expression "Expression") `]`  
[PrimaryNoNewArray](ch15-expressions.md#jls-PrimaryNoNewArray "PrimaryNoNewArray") `[` [Expression](ch15-expressions.md#jls-Expression "Expression") `]`  
[ArrayCreationExpressionWithInitializer](ch15-expressions.md#jls-ArrayCreationExpressionWithInitializer "ArrayCreationExpressionWithInitializer") `[` [Expression](ch15-expressions.md#jls-Expression "Expression") `]`


FieldAccess:


[Primary](ch15-expressions.md#jls-Primary "Primary") `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
`super` `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[TypeName](ch06-names.md#jls-TypeName "TypeName") `.` `super` `.` [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")


MethodInvocation:


[MethodName](ch06-names.md#jls-MethodName "MethodName") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`  
[TypeName](ch06-names.md#jls-TypeName "TypeName") `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`  
[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName") `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`  
[Primary](ch15-expressions.md#jls-Primary "Primary") `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`  
`super` `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`  
[TypeName](ch06-names.md#jls-TypeName "TypeName") `.` `super` `.` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier") `(` \[[ArgumentList](ch15-expressions.md#jls-ArgumentList "ArgumentList")\] `)`


ArgumentList:


[Expression](ch15-expressions.md#jls-Expression "Expression") {`,` [Expression](ch15-expressions.md#jls-Expression "Expression")}


MethodReference:


[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName") `::` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[Primary](ch15-expressions.md#jls-Primary "Primary") `::` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType") `::` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
`super` `::` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[TypeName](ch06-names.md#jls-TypeName "TypeName") `.` `super` `::` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] [Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
[ClassType](ch04-types-values-variables.md#jls-ClassType "ClassType") `::` \[[TypeArguments](ch04-types-values-variables.md#jls-TypeArguments "TypeArguments")\] `new`  
[ArrayType](ch04-types-values-variables.md#jls-ArrayType "ArrayType") `::` `new`


Expression:


[LambdaExpression](ch15-expressions.md#jls-LambdaExpression "LambdaExpression")  
[AssignmentExpression](ch15-expressions.md#jls-AssignmentExpression "AssignmentExpression")


LambdaExpression:


[LambdaParameters](ch15-expressions.md#jls-LambdaParameters "LambdaParameters") `->` [LambdaBody](ch15-expressions.md#jls-LambdaBody "LambdaBody")


LambdaParameters:


`(` \[[LambdaParameterList](ch15-expressions.md#jls-LambdaParameterList "LambdaParameterList")\] `)`  
[ConciseLambdaParameter](ch15-expressions.md#jls-ConciseLambdaParameter "ConciseLambdaParameter")


LambdaParameterList:


[NormalLambdaParameter](ch15-expressions.md#jls-NormalLambdaParameter "NormalLambdaParameter") {`,` [NormalLambdaParameter](ch15-expressions.md#jls-NormalLambdaParameter "NormalLambdaParameter")}  
[ConciseLambdaParameter](ch15-expressions.md#jls-ConciseLambdaParameter "ConciseLambdaParameter") {`,` [ConciseLambdaParameter](ch15-expressions.md#jls-ConciseLambdaParameter "ConciseLambdaParameter")}


NormalLambdaParameter:


{[VariableModifier](ch08-classes.md#jls-VariableModifier "VariableModifier")} [LambdaParameterType](ch15-expressions.md#jls-LambdaParameterType "LambdaParameterType") [VariableDeclaratorId](ch08-classes.md#jls-VariableDeclaratorId "VariableDeclaratorId")  
[VariableArityParameter](ch08-classes.md#jls-VariableArityParameter "VariableArityParameter")


LambdaParameterType:


[UnannType](ch08-classes.md#jls-UnannType "UnannType")  
`var`


ConciseLambdaParameter:


[Identifier](ch03-lexical-structure.md#jls-Identifier "Identifier")  
`_`


LambdaBody:


[Expression](ch15-expressions.md#jls-Expression "Expression")  
[Block](ch14-blocks-statements-patterns.md#jls-Block "Block")


AssignmentExpression:


[ConditionalExpression](ch15-expressions.md#jls-ConditionalExpression "ConditionalExpression")  
[Assignment](ch15-expressions.md#jls-Assignment "Assignment")


Assignment:


[LeftHandSide](ch15-expressions.md#jls-LeftHandSide "LeftHandSide") [AssignmentOperator](ch15-expressions.md#jls-AssignmentOperator "AssignmentOperator") [Expression](ch15-expressions.md#jls-Expression "Expression")


LeftHandSide:


[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName")  
[FieldAccess](ch15-expressions.md#jls-FieldAccess "FieldAccess")  
[ArrayAccess](ch15-expressions.md#jls-ArrayAccess "ArrayAccess")


AssignmentOperator:


(one of)  

``` screen
=  *=  /=  %=  +=  -=  <<=  >>=  >>>=  &=  ^=  |=
```


ConditionalExpression:


[ConditionalOrExpression](ch15-expressions.md#jls-ConditionalOrExpression "ConditionalOrExpression")  
[ConditionalOrExpression](ch15-expressions.md#jls-ConditionalOrExpression "ConditionalOrExpression") `?` [Expression](ch15-expressions.md#jls-Expression "Expression") `:` [ConditionalExpression](ch15-expressions.md#jls-ConditionalExpression "ConditionalExpression")  
[ConditionalOrExpression](ch15-expressions.md#jls-ConditionalOrExpression "ConditionalOrExpression") `?` [Expression](ch15-expressions.md#jls-Expression "Expression") `:` [LambdaExpression](ch15-expressions.md#jls-LambdaExpression "LambdaExpression")  


ConditionalOrExpression:


[ConditionalAndExpression](ch15-expressions.md#jls-ConditionalAndExpression "ConditionalAndExpression")  
[ConditionalOrExpression](ch15-expressions.md#jls-ConditionalOrExpression "ConditionalOrExpression") `||` ConditionalAndExpression


ConditionalAndExpression:


[InclusiveOrExpression](ch15-expressions.md#jls-InclusiveOrExpression "InclusiveOrExpression")  
[ConditionalAndExpression](ch15-expressions.md#jls-ConditionalAndExpression "ConditionalAndExpression") `&&` [InclusiveOrExpression](ch15-expressions.md#jls-InclusiveOrExpression "InclusiveOrExpression")


InclusiveOrExpression:


[ExclusiveOrExpression](ch15-expressions.md#jls-ExclusiveOrExpression "ExclusiveOrExpression")  
[InclusiveOrExpression](ch15-expressions.md#jls-InclusiveOrExpression "InclusiveOrExpression") `|` [ExclusiveOrExpression](ch15-expressions.md#jls-ExclusiveOrExpression "ExclusiveOrExpression")


ExclusiveOrExpression:


[AndExpression](ch15-expressions.md#jls-AndExpression "AndExpression")  
[ExclusiveOrExpression](ch15-expressions.md#jls-ExclusiveOrExpression "ExclusiveOrExpression") `^` [AndExpression](ch15-expressions.md#jls-AndExpression "AndExpression")


AndExpression:


[EqualityExpression](ch15-expressions.md#jls-EqualityExpression "EqualityExpression")  
[AndExpression](ch15-expressions.md#jls-AndExpression "AndExpression") `&` [EqualityExpression](ch15-expressions.md#jls-EqualityExpression "EqualityExpression")


EqualityExpression:


[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression")  
[EqualityExpression](ch15-expressions.md#jls-EqualityExpression "EqualityExpression") `==` [RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression")  
[EqualityExpression](ch15-expressions.md#jls-EqualityExpression "EqualityExpression") `!=` [RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression")


RelationalExpression:


[ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression")  
[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression") `<` [ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression")  
[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression") `>` [ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression")  
[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression") `<=` [ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression")  
[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression") `>=` [ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression")  
[InstanceofExpression](ch15-expressions.md#jls-InstanceofExpression "InstanceofExpression")


InstanceofExpression:


[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression") `instanceof` [ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType")  
[RelationalExpression](ch15-expressions.md#jls-RelationalExpression "RelationalExpression") `instanceof` [Pattern](ch14-blocks-statements-patterns.md#jls-Pattern "Pattern")


ShiftExpression:


[AdditiveExpression](ch15-expressions.md#jls-AdditiveExpression "AdditiveExpression")  
[ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression") `<<` [AdditiveExpression](ch15-expressions.md#jls-AdditiveExpression "AdditiveExpression")  
[ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression") `>>` [AdditiveExpression](ch15-expressions.md#jls-AdditiveExpression "AdditiveExpression")  
[ShiftExpression](ch15-expressions.md#jls-ShiftExpression "ShiftExpression") `>>>` [AdditiveExpression](ch15-expressions.md#jls-AdditiveExpression "AdditiveExpression")


AdditiveExpression:


[MultiplicativeExpression](ch15-expressions.md#jls-MultiplicativeExpression "MultiplicativeExpression")  
[AdditiveExpression](ch15-expressions.md#jls-AdditiveExpression "AdditiveExpression") `+` [MultiplicativeExpression](ch15-expressions.md#jls-MultiplicativeExpression "MultiplicativeExpression")  
[AdditiveExpression](ch15-expressions.md#jls-AdditiveExpression "AdditiveExpression") `-` [MultiplicativeExpression](ch15-expressions.md#jls-MultiplicativeExpression "MultiplicativeExpression")


MultiplicativeExpression:


[UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
[MultiplicativeExpression](ch15-expressions.md#jls-MultiplicativeExpression "MultiplicativeExpression") `*` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
[MultiplicativeExpression](ch15-expressions.md#jls-MultiplicativeExpression "MultiplicativeExpression") `/` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
[MultiplicativeExpression](ch15-expressions.md#jls-MultiplicativeExpression "MultiplicativeExpression") `%` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")


UnaryExpression:


[PreIncrementExpression](ch15-expressions.md#jls-PreIncrementExpression "PreIncrementExpression")  
[PreDecrementExpression](ch15-expressions.md#jls-PreDecrementExpression "PreDecrementExpression")  
`+` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
`-` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
[UnaryExpressionNotPlusMinus](ch15-expressions.md#jls-UnaryExpressionNotPlusMinus "UnaryExpressionNotPlusMinus")


PreIncrementExpression:


`++` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")


PreDecrementExpression:


`--` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")


UnaryExpressionNotPlusMinus:


[PostfixExpression](ch15-expressions.md#jls-PostfixExpression "PostfixExpression")  
`~` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
`!` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
[CastExpression](ch15-expressions.md#jls-CastExpression "CastExpression")  
[SwitchExpression](ch15-expressions.md#jls-SwitchExpression "SwitchExpression")


PostfixExpression:


[Primary](ch15-expressions.md#jls-Primary "Primary")  
[ExpressionName](ch06-names.md#jls-ExpressionName "ExpressionName")  
[PostIncrementExpression](ch15-expressions.md#jls-PostIncrementExpression "PostIncrementExpression")  
[PostDecrementExpression](ch15-expressions.md#jls-PostDecrementExpression "PostDecrementExpression")


PostIncrementExpression:


[PostfixExpression](ch15-expressions.md#jls-PostfixExpression "PostfixExpression") `++`


PostDecrementExpression:


[PostfixExpression](ch15-expressions.md#jls-PostfixExpression "PostfixExpression") `--`


CastExpression:


`(` [PrimitiveType](ch04-types-values-variables.md#jls-PrimitiveType "PrimitiveType") `)` [UnaryExpression](ch15-expressions.md#jls-UnaryExpression "UnaryExpression")  
`(` [ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType") {[AdditionalBound](ch04-types-values-variables.md#jls-AdditionalBound "AdditionalBound")} `)` [UnaryExpressionNotPlusMinus](ch15-expressions.md#jls-UnaryExpressionNotPlusMinus "UnaryExpressionNotPlusMinus")  
`(` [ReferenceType](ch04-types-values-variables.md#jls-ReferenceType "ReferenceType") {[AdditionalBound](ch04-types-values-variables.md#jls-AdditionalBound "AdditionalBound")} `)` [LambdaExpression](ch15-expressions.md#jls-LambdaExpression "LambdaExpression")  


SwitchExpression:


`switch` `(` [Expression](ch15-expressions.md#jls-Expression "Expression") `)` [SwitchBlock](ch14-blocks-statements-patterns.md#jls-SwitchBlock "SwitchBlock")


