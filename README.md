# `sqlx-bootstrap`

Supporting library for [`sqlx`](https://github.com/launchbadge/sqlx) that will check for database existance on start-up and create a user and database if needed.

## When To Use

When building a service and deploying it, one of the manual steps is to log in to the server and create a new database, optionally with a new user for it. If you deploy new services often, this can be cumbersome and prone to mistakes.

`sqlx-bootstrap` instead allows you to define the expected database configuration at startup of your server. When it runs in a new environment, it will automatically connect to the database instance and create a new role and database for the service. On subsequent runs, it will just do nothing.

This means you don't need to do anything manually, just start up the application and it will create the required resources.

## Examples

A basic example of library usage can be found in the `examples/basic` directory which will show you how to instantiate the `BootstrapConfig` struct and run the bootstrapping process.

## Features

The library currently only supports the `sqlx` library running on the `tokio` runtime for a `postgres` database.
