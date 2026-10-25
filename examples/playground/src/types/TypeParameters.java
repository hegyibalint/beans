package types;

// Navigate bounds, a generic superclass, and implemented interfaces.
// The fields below should resolve `T` to this type parameter.
public class TypeParameters<T extends ExternalImportedType & InterfaceBound & AdditionalInterfaceBound>
        extends GenericType<ExternalImportedType> implements InterfaceBound, AdditionalInterfaceBound {
    // Navigate to the bounded type parameter.
    // Go to Definition on `T` should land on `T` in the class declaration.
    T parameter;
    // Hover an array of the type parameter.
    // Hovering `T` should show `T[]`.
    T[] parameterArray;

    // Hover an upper bound using the type parameter.
    // Hovering `T` should show `T`; Go to Definition should reach its declaration.
    GenericType<? extends T> upperBound;
    // Hover a lower bound using the type parameter.
    // Hovering `T` should show `T`; Go to Definition should reach its declaration.
    GenericType<? super T> lowerBound;
}

interface InterfaceBound {}

interface AdditionalInterfaceBound {}
