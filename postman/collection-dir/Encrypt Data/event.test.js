// Parse the response body
var response = pm.response.json();

// Set the value of 'ff' to a variable named 'ff_value'
pm.environment.set("ff_value", response.data.ff);