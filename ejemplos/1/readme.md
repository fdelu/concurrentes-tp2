# Ejemplo 1

En este ejemplo se van a abrir 3 servidores con 2 cafeteras:

- Servidor 1 (127.0.0.1) con una cafetera
- Servidor 2 (127.0.0.2) con otra cafetera
- Servidor 3 (127.0.0.3)

## Ejecución

Ejecutar `./start.sh`. En la terminal se deberían imprimir los outputs de las cafeteras, y también quedán logs de ellas en `cafeteras/logs` y de los servidores en `servidores/logs`.

## Resultado esperado

La cantidad de puntos inicial para cada usuario es 100.

Todas las ordenes de ambas cafeteras deberían tener éxito, excepto la del café 2 de la cafetera 1 y la del café 3 de la cafetera 2, ya que no alcanzan los puntos.

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
