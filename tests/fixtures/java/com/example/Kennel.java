package com.example;

import java.util.List;
import java.util.ArrayList;

public class Kennel {
    private List<Dog> dogs;
    private String name;

    public Kennel(String name) {
        this.name = name;
        this.dogs = new ArrayList<>();
    }

    public void addDog(Dog dog) {
        dogs.add(dog);
    }

    public Dog findDog(String name) {
        for (Dog dog : dogs) {
            if (dog.getName().equals(name)) {
                return dog;
            }
        }
        return null;
    }
}
