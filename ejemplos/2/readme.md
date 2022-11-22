# Ejemplo 2

En este ejemplo se van a abrir 3 servidores con 2 cafeteras:

- Servidor 1 (127.0.0.1) con una cafetera
- Servidor 2 (127.0.0.2) con otra cafetera
- Servidor 3 (127.0.0.3) con otra cafetera

## Ejecución

Ejecutar `./start.sh`. En la terminal se deberían imprimir los outputs de las cafeteras, y también quedán logs de ellas en `cafeteras/logs` y de los servidores en `servidores/logs`.

## Resultado esperado

Este ejemplo demuestra una cafetera que se conecta despues de que la red ya haya hecho transacciones. La secuencia es identica al ejemplo 1 pero al finalizar estas se conecta la tercera cafetera al servidor 3 y intentará hacer una transaccion que cuesta 100 puntos para el usuario 3, la cual debería fallar porque la cafetera 1 ya le descontó 5 puntos al usuario 3 y luego una transaccion que cuesta 90 puntos al usuario 3 la cual debería ser exitosa.

### Cafetera 1

```
[SALE: coffee 'cafe 1', user '3', '5' points]: Completed
[SALE: coffee 'cafe 2', user '3', '150' points]: Failed: InsufficientPoints
[SALE: coffee 'cafe 3', user '5', '15' points]: Completed
[SALE: coffee 'cafe 4', user '5', '25' points]: Completed
[RECHARGE: user '5', '50' points]: Completed
```

### Cafetera 2

```
[SALE: coffee 'cafe 1', user '1', '20' points]: Completed
[SALE: coffee 'cafe 2', user '1', '40' points]: Completed
[SALE: coffee 'cafe 3', user '1', '60' points]: Failed: InsufficientPoints
[SALE: coffee 'cafe 4', user '1', '10' points]: Completed
[RECHARGE: user '25', '10' points]: Completed
```

### Cafetera 3

```
[SALE: coffee 'cafe 5', user '3', '100' points]: Failed: InsufficientPoints
[SALE: coffee 'cafe 6', user '3', '90' points]: Completed
```

### Base de datos

TODO: Ver logs o dump de db.
