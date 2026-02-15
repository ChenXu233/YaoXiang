---
title: 'Chapter 1: Hello, Program'
---

# Chapter 1: Hello, Program

> **Chapter Goal**: Understand what a program is, and how code becomes a running program

## 1.1 Starting with a Question

Have you ever wondered how the apps on your phone, the buttons on websites, and the games you play work?

The answer is: **They are all programs**.

A program is a series of **instructions** for the computer. Just like a recipe tells a chef how to cook, a program tells the computer how to do things.

## 1.2 What is Code?

**Code** is a program written in a programming language.

We speak Chinese and English in daily life, programmers "speak" with computers using programming languages. YaoXiang is a programming language.

```yaoxiang
# This is a line of YaoXiang code
print("Hello, 世界")
```

This line of code means: **display "Hello, 世界" on the screen**.

> **Tip**: Text after `#` in code is a "comment", computers don't execute them, they're just explanations for humans.

## 1.3 From Code to Program

Humans and computers use different "languages", so the code we write needs to go through a "translation" process:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Source Code │ ──▶ │  Compiler   │ ──▶ │Executable  │
│  (YaoXiang)│     │ Translator  │     │(Computer)  │
└─────────────┘     └─────────────┘     └─────────────┘
```

This "translator" is called a **compiler**. YaoXiang's compiler is called `yaoxiang`.

## 1.4 Your First Program

Let's write your first YaoXiang program!

```yaoxiang
# hello.yx
main: () -> Void = {
    print("Hello, I am your first program!")
}
```

**Line-by-line explanation:**

| Code | Meaning |
|------|---------|
| `main:` | This is a function called "main", the program starts executing here |
| `()` | This function takes no input parameters |
| `-> Void` | This function doesn't return any value |
| `{ ... }` | The content in braces is the instruction the function executes |
| `print("...")` | Display the text in quotes on the screen |

> **Note**: YaoXiang code must use **4 spaces** for indentation, not Tab!

## 1.5 Running the Program

Save the code above as `hello.yx`, then run it in terminal:

```bash
yaoxiang hello.yx
```

You will see on the screen:

```
Hello, I am your first program!
```

**Congratulations! You've written and run your first program!** 🎉

## 1.6 Program Structure

Let's look at the complete structure of the program again:

```yaoxiang
# Filename: hello.yx
main: () -> Void = {
    # Write instructions here
    print("Hello, 世界")
}
```

- **Function**: An independent unit of functionality
- **main**: Program entry point, all programs start here
- **print**: YaoXiang's built-in "print" functionality

## 1.7 Chapter Summary

| Concept | Understanding |
|---------|----------------|
| Program | A series of instructions for the computer |
| Code | A program written in a programming language |
| Compiler | Translates code into a language the computer understands |
| Function | Basic functional unit of a program |
| main | Entry point of the program |

## 1.8 I Ching Introduction

> "The Changes has the same standard as heaven and earth, therefore it can encompass all the ways of heaven and earth."
> — "Xici Zhuan", Book of Changes
>
> Just as the I Ching uses yin-yang hexagrams to describe all things in heaven and earth, programs use code instructions to describe the way of computation. From a string of characters to a running program, this itself is a kind of "Tao".
