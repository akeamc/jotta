# Jotta

A collection of third-party Jottacloud clients of varying levels of abstraction.

## Building

All Dockerfiles use the root of the repository as their build context, and must
therefore be built like so:

```
docker build -t jotta-rest -f jotta-rest/Dockerfile .
```

This does **NOT** work:

```
cd jotta-rest
docker build -t jotta-rest .
```
