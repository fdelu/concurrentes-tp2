# Ejemplo 5

En este ejemplo se van a abrir 5 servidores con 10 cafeteras cada uno.

## Ejecución

Ejecutar `./start.sh`. Quedan los logs de los servidores en `servidores/logs` (no de las cafeteras ya que se mezclan por usar la misma config). Este script puede tardar un poco más que los anteriores en completarse.

## Resultado esperado

La cantidad de puntos inicial para cada usuario es 100, y en las ordenes de cada cafetera se canjea un café de 1 punto, 2 veces por cada cafetera para cada usuario (ids del 0 al 9).

En total entonces se esperaría que se reste la totalidad de los 100 puntos para cada uno de ellos usuarios. Esto se puede corroborar viendo el dump de cada servidor en `serivdores/databases` luego de ejecutar el script:

```json
{
  "2": 0,
  "5": 0,
  "6": 0,
  "3": 0,
  "7": 0,
  "0": 0,
  "1": 0,
  "8": 0,
  "4": 0,
  "9": 0
}
```
