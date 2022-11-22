# Ejemplo 3

En este ejemplo se van a abrir 3 servidores con 2 cafeteras:

- Servidor 1 (127.0.0.1) con una cafetera
- Servidor 2 (127.0.0.2) con otra cafetera
- Servidor 3 (127.0.0.3) con tra cafetera

## Ejecución

Ejecutar `./start.sh`. En la terminal se deberían imprimir los outputs de las cafeteras, y también quedán logs de ellas en `cafeteras/logs` y de los servidores en `servidores/logs`.

## Resultado esperado

En este ejemplo todas las cafeteras intenan hacer una compra de 55 puntos con el usuario 3 pero al haber solo 100 puntos, solo uno de los servidores lo logrará.

### Una de las cafeteras

```
[SALE: coffee 'cafe 1', user '3', '55' points]: Completed
```

### Las otras dos

```
[SALE: coffee 'cafe 1', user '3', '55' points]: Failed: InsufficientPoints
```

### Base de datos

TODO: Ver logs o dump de db.
