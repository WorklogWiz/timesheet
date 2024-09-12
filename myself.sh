#
# Retrieves the information about the current user with the /myself resource
# You must insert your jira security token
curl --request GET \
  --url 'https://autostore.atlassian.net/rest/api/2/myself' \
  --user 'steinar.cook@autostoresystem.com:<Jira security token goes here>' \
  --header 'Accept: application/json'
