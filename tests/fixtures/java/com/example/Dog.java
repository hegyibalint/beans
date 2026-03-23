package com.example;

public class Dog implements Animal {
    private String name;
    private int age;

    public Dog(String name, int age) {
        this.name = name;
        this.age = age;
    }

    @Override
    public String getName() {
        return name;
    }

    @Override
    public void makeSound() {
        System.out.println("Woof!");
    }

    public int getAge() {
        return age;
    }
}
