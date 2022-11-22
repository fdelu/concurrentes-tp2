# Ejemplo 3

En este ejemplo se van a abrir 3 servidores con 2 cafeteras:

- Servidor 1 (127.0.0.1) con una cafetera
- Servidor 2 (127.0.0.2) con otra cafetera
- Servidor 3 (127.0.0.3)

El objetivo de este es mostrar que dos servidores distintos pueden hacer transacciones sobre el mismo usuario concurrentemente sin inconvenientes.

## Ejecución

Ejecutar `./start.sh`. En la terminal se deberían imprimir los outputs de las cafeteras, y también quedán logs de ellas en `cafeteras/logs` y de los servidores en `servidores/logs`.

## Resultado esperado

Todas las transacciones de ambas cafeteras deberán ser exitosas excepto por el cafe 5 de la cafetera 1 y el cafe 7 de la cafetera 2 ya que para ellas no alcanzaran los puntos.

### Cafetera 1

```
[SALE: coffee 'cafe 1', user '3', '5' points]: Completed
[SALE: coffee 'cafe 2', user '3', '5' points]: Completed
[SALE: coffee 'cafe 5', user '3', '100' points]: Failed: InsufficientPoints
[SALE: coffee 'cafe 6', user '3', '5' points]: Completed
```

### Cafetera 2

```
[SALE: coffee 'cafe 3', user '3', '5' points]: Completed
[SALE: coffee 'cafe 4', user '3', '5' points]: Completed
[SALE: coffee 'cafe 7', user '3', '100' points]: Failed: InsufficientPoints
[SALE: coffee 'cafe 8', user '3', '5' points]: Completed
```

### Base de datos

TODO: Ver logs o dump de db.
