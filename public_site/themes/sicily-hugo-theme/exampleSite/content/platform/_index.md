---
title: "Platform"
subtitle: "How our platform works"
# meta description
description: "How our platform works guide"
draft: false
---

# Introduction

## About this guide

 

*Robot Framework Quick Start Guide* introduces the most important `Robot

Framework <http://robotframework.org>`_ features. You can simply browse

through it and look at the examples, but you can also use the guide as an

`executable demo`__. All features shown here are explained more thoroughly

in `Robot Framework User Guide`_.

  

__ `Executing this guide`_

.. _Robot Framework User Guide: http://robotframework.org/robotframework/#user-guide

  

## Robot Framework overview


  

`Robot Framework`_ is a generic open source test automation framework for

acceptance testing and acceptance test-driven development (ATDD). It has

easy-to-use tabular test data syntax and it utilizes the keyword-driven

testing approach. Its testing capabilities can be extended by test libraries

implemented either with Python or Java, and users can create new higher-level

keywords from existing ones using the same syntax that is used for creating

test cases.

  

Robot Framework is operating system and application independent. The core

framework is implemented using `Python <http://python.org>`_ and runs also on

`Jython <http://jython.org>`_ (JVM) and `IronPython <http://ironpython.net>`_

(.NET). The framework has a rich ecosystem around it consisting of various

generic test libraries and tools that are developed as separate projects.

  

For more information about Robot Framework and the ecosystem, see

http://robotframework.org. There you can find plenty more documentation,

demo projects, list of available test libraries and other tools, and so on.

  

## Demo application

  

The sample application for this guide is a variation on a classic login

example: it is a command-line based authentication server written in Python.

The application allows a user to do three things:

  

- Create an account with a valid password.

- Log in with a valid user name and password.

- Change the password of an existing account.

  

The application itself is in `<sut/login.py>`_ file and can be executed with

a command `python sut/login.py`. Attempting to log in with a non-existent

user account or with an invalid password results in the same error message::

  

> python sut/login.py login nobody P4ssw0rd

Access Denied

  

After creating a user account with valid password login succeeds::

  

> python sut/login.py create fred P4ssw0rd

SUCCESS

  

> python sut/login.py login fred P4ssw0rd

Logged In

  

There are two requirements that a password must fulfill to be valid: it must

be between 7-12 characters long, and it must contain lower and upper case

letters and numbers, but it must not contain special characters. Trying to

create a user with invalid password fails::

  

> python sut/login.py create fred short

Creating user failed: Password must be 7-12 characters long

  

> python sut/login.py create fred invalid

Creating user failed: Password must be a combination of lowercase and

uppercase letters and numbers

  

Changing password with invalid credentials results in the same error message

as logging in with invalid credentials. The validity of new password is

verified and if not valid, an error message is given::

  

> python sut/login.py change-password fred wrong NewP4ss

Changing password failed: Access Denied

  

> python sut/login.py change-password fred P4ssw0rd short

Changing password failed: Password must be 7-12 characters long

  

> python sut/login.py change-password fred P4ssw0rd NewP4ss

SUCCESS

  

The application uses a simple database file to keep track on user statuses.

The file is located in operating system dependent temporary directory.
